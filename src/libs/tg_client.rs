use crate::libs::tg_structs::{
    TgDialogOutputItem, TgMessageOutputItem, TgParticipantOutputItem, TgPeerOutput,
};
use anyhow::{Context, Ok, Result};
use grammers_client::{
    Client, SenderPool, SignInError,
    message::Message,
    peer::{Peer, Role},
};
use grammers_client::{message, tl};
use grammers_session::storages::SqliteSession;
use grammers_tl_types as tl_types;
use rand;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TgClient {
    client: Mutex<Client>,
    _session: Arc<SqliteSession>,
}

impl TgClient {
    pub async fn new(
        api_id: i32,
        api_hash: &str,
        phone_num: &str,
        session_path: &str,
    ) -> Result<Self> {
        let session = Arc::new(SqliteSession::open(session_path).await?);
        let SenderPool { runner, handle, .. } = SenderPool::new(Arc::clone(&session), api_id);
        let client = Client::new(handle);
        _ = tokio::spawn(runner.run());
        if !client.is_authorized().await? {
            let token = client.request_login_code(phone_num, api_hash).await?;
            let code = prompt("Enter the code you received: ")?;
            let signed_in = client.sign_in(&token, &code).await;
            match signed_in {
                Err(SignInError::PasswordRequired(password_token)) => {
                    let hint = password_token.hint().unwrap();
                    let prompt_message = format!("Enter the password (hint {}): ", &hint);
                    let password = prompt(prompt_message.as_str())?;
                    client.check_password(password_token, password).await?;
                }
                Result::Ok(_) => (),
                Err(e) => panic!("{}", e),
            };
        }
        Ok(Self {
            client: Mutex::new(client),
            _session: session,
        })
    }

    pub async fn is_authorized(&self) -> Result<bool> {
        let client = self.client.lock().await;
        Ok(client.is_authorized().await?)
    }

    pub async fn get_dialogs(&self) -> Result<Vec<TgDialogOutputItem>> {
        let client = self.client.lock().await;
        let mut dialogs_it = client.iter_dialogs();
        let mut dialog_out: Vec<TgDialogOutputItem> = Vec::new();
        while let Some(dialog) = dialogs_it.next().await? {
            let peer = dialog.peer();
            match peer {
                Peer::User(u) => {
                    let user_id = u.id().bare_id();
                    let user_name = u.username().map(|x| x.to_string());
                    let user_full_name = u.full_name();
                    let u_struct = TgDialogOutputItem {
                        dialog_id: user_id,
                        dialog_name: user_name,
                        dialog_full_name: Some(user_full_name),
                        dialog_type: "user".to_string(),
                    };
                    dialog_out.push(u_struct);
                }
                Peer::Group(g) => {
                    let group_id = g.id().bare_id();
                    let group_name = g.username().map(|x| x.to_string());
                    let group_title = g.title().map(|x| x.to_string());
                    let g_struct = TgDialogOutputItem {
                        dialog_id: group_id,
                        dialog_name: group_name,
                        dialog_full_name: group_title,
                        dialog_type: "group".to_string(),
                    };
                    dialog_out.push(g_struct);
                }
                Peer::Channel(c) => {
                    let ch_id = c.id().bare_id();
                    let ch_name = c.username().map(|x| x.to_string());
                    let ch_full_name = c.title().to_string();
                    let c_struct = TgDialogOutputItem {
                        dialog_id: ch_id,
                        dialog_name: ch_name,
                        dialog_full_name: Some(ch_full_name),
                        dialog_type: "channel".to_string(),
                    };
                    dialog_out.push(c_struct);
                }
            }
        }
        Ok(dialog_out)
    }

    pub async fn search_peer(&self, query: String, limit: usize) -> Result<Vec<TgPeerOutput>> {
        let mut result: Vec<TgPeerOutput> = Vec::new();
        let client = self.client.lock().await;
        let found_items = client.search_peer(query.as_str(), limit).await?;
        for item in found_items {
            let peer = item.peer();
            result.push(self.to_peer_output(peer).await);
        }
        Ok(result)
    }

    pub async fn get_peer_info(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<TgPeerOutput> {
        let my_peer = self.get_peer(kind, username, id).await?;
        let p_output = self.to_peer_output(&my_peer).await;
        Ok(p_output)
    }

    async fn get_peer(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<Peer> {
        let peer = match kind.as_str() {
            "username" => {
                let u_name = username
                    .filter(|x| !x.trim().is_empty())
                    .ok_or_else(|| anyhow::anyhow!("username is required when kind='username'"))?;
                self.get_peer_by_name(u_name).await
            }
            "id" => {
                let peer_id = id.ok_or_else(|| anyhow::anyhow!("id is required when kind='id'"))?;
                self.get_peer_by_id(peer_id).await
            }
            other => Err(anyhow::anyhow!(
                "unsupported peer lookup kind: {} (expected 'username' or 'id')",
                other
            )),
        }?;
        Ok(peer)
    }

    async fn get_peer_by_id(&self, peer_id: i64) -> Result<Peer> {
        let client = self.client.lock().await;
        let mut dialogs_iter = client.iter_dialogs();
        while let Some(dialog) = dialogs_iter.next().await? {
            let current_peer = dialog.peer();
            if current_peer.id().bare_id() == peer_id {
                return Ok(current_peer.clone());
            }
        }
        Err(anyhow::anyhow!("peer with id {} not found", peer_id))
    }

    async fn get_peer_by_name(&self, peer_name: String) -> Result<Peer> {
        let client = self.client.lock().await;
        let peer = client
            .resolve_username(peer_name.as_str().trim_start_matches("&"))
            .await?
            .ok_or_else(|| anyhow::anyhow!(format!("no peer with {} username", peer_name)))?;
        Ok(peer.clone())
    }

    pub async fn get_messages_count(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<usize> {
        let my_peer = self.get_peer(kind, username, id).await?;
        let my_peer_ref = my_peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let client = self.client.lock().await;
        let mut messages = client.iter_messages(my_peer_ref);
        let messages_count = messages.total().await?;
        Ok(messages_count)
    }

    pub async fn get_search_messages_count(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        query: String,
    ) -> Result<usize> {
        let my_peer = self.get_peer(kind, username, id).await?;
        let my_peer_ref = my_peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let client = self.client.lock().await;
        let mut search_messages = client.search_messages(my_peer_ref).query(query.as_str());
        let search_count = search_messages.total().await?;
        Ok(search_count)
    }

    pub async fn get_participants_count(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<usize> {
        let my_peer = self.get_peer(kind, username, id).await?;
        let my_peer_ref = my_peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let client = self.client.lock().await;
        let mut participants = client.iter_participants(my_peer_ref);
        let participants_count = participants.total().await?;
        Ok(participants_count)
    }

    pub async fn get_messages(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<TgMessageOutputItem>> {
        let mut result: Vec<TgMessageOutputItem> = Vec::new();
        let peer = self.get_peer(kind, username, id).await?;
        let peer_ref = peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let client = self.client.lock().await;
        let mut messages = client.iter_messages(peer_ref);
        let total = messages.total().await?;
        let limit = limit.min(total);
        let mut cnt = 0;
        while let Some(msg) = messages.next().await? {
            match msg.sender() {
                Some(sender_peer) => {
                    let m = self.to_message_struct(&msg, sender_peer).await?;
                    result.push(m);
                }
                None => {
                    let m = self.to_message_struct(&msg, &peer).await?;
                    result.push(m);
                }
            }
            cnt += 1;
            if cnt >= limit {
                break;
            }
        }
        result.reverse();
        Ok(result)
    }

    pub async fn get_search_messages(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        query: String,
        limit: usize,
    ) -> Result<Vec<TgMessageOutputItem>> {
        let mut result: Vec<TgMessageOutputItem> = Vec::new();
        let peer = self.get_peer(kind, username, id).await?;
        let my_peer = peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let client = self.client.lock().await;
        let mut search_messages = client.search_messages(my_peer).query(query.as_str());
        let mut cnt = 0;
        while let Some(msg) = search_messages.next().await? {
            match msg.sender() {
                Some(sender_peer) => {
                    let m = self.to_message_struct(&msg, sender_peer).await?;
                    result.push(m);
                }
                None => {
                    let m = self.to_message_struct(&msg, &peer).await?;
                    result.push(m);
                }
            }
            cnt += 1;
            if cnt >= limit {
                break;
            }
        }
        result.reverse();
        Ok(result)
    }

    pub async fn get_participants(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<TgParticipantOutputItem>> {
        let mut result: Vec<TgParticipantOutputItem> = Vec::new();
        let peer = self.get_peer(kind, username, id).await?;
        let my_peer = peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let client = self.client.lock().await;
        let mut participants = client.iter_participants(my_peer);
        let total = participants.total().await?;
        let limit = limit.min(total);
        let mut cnt = 0;
        while let Some(participant) = participants.next().await? {
            let p_role = match participant.role {
                Role::User(_) => "user",
                Role::Creator(_) => "creator",
                Role::Admin(_) => "admin",
                Role::Banned(_) => "banned",
                Role::Left(_) => "left",
                _ => "unknown",
            }
            .to_string();
            let p_item = TgParticipantOutputItem {
                id: participant.user.id().bare_id(),
                full_name: participant.user.full_name(),
                username: participant.user.username().map(|x| x.to_string()),
                is_bot: participant.user.is_bot(),
                is_premium: participant.user.is_premium(),
                phone_number: participant.user.phone().map(|x| x.to_string()),
                role: p_role,
            };
            result.push(p_item);
            cnt += 1;
            if cnt >= limit {
                break;
            }
        }
        Ok(result)
    }

    pub async fn send_message(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        msg: String,
        msg_reply_to: Option<i32>,
    ) -> Result<()> {
        let peer = self.get_peer(kind, username, id).await?;
        let client = self.client.lock().await;
        let peer_ref = peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;

        let to_send = match msg_reply_to {
            Some(reply_to) if reply_to > 0 => message::InputMessage::new()
                .text(msg.as_str())
                .reply_to(Some(reply_to)),
            Some(reply_to) => {
                return Err(anyhow::anyhow!(
                    "msg_reply_to must be a positive message id, got {}",
                    reply_to
                ));
            }
            None => message::InputMessage::new().text(msg.as_str()),
        };
        client.send_message(peer_ref, to_send).await?;
        Ok(())
    }

    pub async fn add_post_comment(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        message_id: i32,
        post_comment: String,
    ) -> Result<()> {
        let peer = self.get_peer(kind, username, id).await?;
        let peer_ref = peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let input_peer = tl::enums::InputPeer::from(peer_ref);
        let discussion_req = tl::functions::messages::GetDiscussionMessage {
            peer: input_peer,
            msg_id: message_id,
        };
        let client = self.client.lock().await;
        let d_messages = client.invoke(&discussion_req).await?;
        let discussion = match d_messages {
            tl::enums::messages::DiscussionMessage::Message(m) => m,
        };
        let disc_msg_id = discussion.messages.first().and_then(|msg| match msg {
            tl::enums::Message::Message(m) => Some(m.id),
            _ => None,
        });
        let discussion_msg_id = disc_msg_id.ok_or_else(|| {
            anyhow::anyhow!(format!(
                "failed to resolve discussion message id for {} message id",
                message_id
            ))
        })?;
        let disc_msg_input_peer = discussion.chats.first().and_then(|ch| match ch {
            tl::enums::Chat::Channel(c) => {
                Some(tl::enums::InputPeer::Channel(tl::types::InputPeerChannel {
                    channel_id: c.id,
                    access_hash: c.access_hash.unwrap_or(0),
                }))
            }
            tl::enums::Chat::Chat(c) => {
                Some(tl::enums::InputPeer::Chat(tl::types::InputPeerChat {
                    chat_id: c.id,
                }))
            }
            _ => None,
        });
        let discussion_input_peer = disc_msg_input_peer.ok_or_else(|| {
            anyhow::anyhow!(format!(
                "failed to resolve discussion message peer for {} message id",
                message_id
            ))
        })?;

        let comment_req = tl::functions::messages::SendMessage {
            no_webpage: false,
            silent: false,
            background: false,
            clear_draft: false,
            noforwards: false,
            update_stickersets_order: false,
            invert_media: false,
            peer: discussion_input_peer,
            reply_to: Some(tl::enums::InputReplyTo::Message(
                tl::types::InputReplyToMessage {
                    reply_to_msg_id: discussion_msg_id,
                    top_msg_id: None,
                    reply_to_peer_id: None,
                    quote_text: None,
                    quote_entities: None,
                    quote_offset: None,
                    monoforum_peer_id: None,
                    todo_item_id: None,
                },
            )),
            message: post_comment,
            random_id: rand::random(),
            reply_markup: None,
            entities: None,
            schedule_date: None,
            send_as: None,
            quick_reply_shortcut: None,
            effect: None,
            allow_paid_floodskip: false,
            allow_paid_stars: None,
            schedule_repeat_period: None,
            suggested_post: None,
        };

        client.invoke(&comment_req).await?;
        Ok(())
    }

    pub async fn get_post_comments(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
        message_id: i32,
        limit: i32,
    ) -> Result<Vec<TgMessageOutputItem>> {
        let mut result: Vec<TgMessageOutputItem> = Vec::new();
        let peer = self.get_peer(kind, username, id).await?;
        let fallback_sender: (Option<i64>, Option<String>, Option<String>) = match &peer {
            Peer::User(u) => (
                Some(u.id().bare_id()),
                u.username().map(|x| x.to_string()),
                Some(u.full_name()),
            ),
            Peer::Group(g) => (
                Some(g.id().bare_id()),
                g.username().map(|x| x.to_string()),
                g.title().map(|x| x.to_string()),
            ),
            Peer::Channel(c) => (
                Some(c.id().bare_id()),
                c.username().map(|x| x.to_string()),
                Some(c.title().to_string()),
            ),
        };
        let peer_ref = peer
            .to_ref()
            .await
            .context("failed to resolve input peer reference")?;
        let input_peer = tl::enums::InputPeer::from(peer_ref);
        let client = self.client.lock().await;
        let replies_request = tl::functions::messages::GetReplies {
            peer: input_peer,
            msg_id: message_id,
            offset_id: 0,
            offset_date: 0,
            add_offset: 0,
            limit,
            max_id: 0,
            min_id: 0,
            hash: 0,
        };
        let replies: tl::enums::messages::Messages = client.invoke(&replies_request).await?;
        let (messages, users, chats) = match replies {
            tl::enums::messages::Messages::Messages(m) => (m.messages, m.users, m.chats),
            tl::enums::messages::Messages::ChannelMessages(m) => (m.messages, m.users, m.chats),
            tl::enums::messages::Messages::Slice(m) => (m.messages, m.users, m.chats),
            tl::enums::messages::Messages::NotModified(_) => (Vec::new(), Vec::new(), Vec::new()),
        };
        for message in messages {
            let tl::enums::Message::Message(msg) = message else {
                continue;
            };
            let (sender_id, sender_username, sender_full_name) = match msg.from_id.as_ref() {
                Some(tl::enums::Peer::User(p)) => {
                    let uid = p.user_id;
                    let user = users.iter().find_map(|u| match u {
                        tl::enums::User::User(user) if user.id == uid => Some(user),
                        _ => None,
                    });
                    let username = user.and_then(|u| u.username.clone());
                    let full_name = user.map(|u| {
                        format!(
                            "{} {}",
                            u.first_name.clone().unwrap_or_default(),
                            u.last_name.clone().unwrap_or_default()
                        )
                    });
                    (Some(uid), username, full_name)
                }
                Some(tl::enums::Peer::Channel(p)) => {
                    let ch_id = p.channel_id;
                    let chan = chats.iter().find_map(|c| match c {
                        tl::enums::Chat::Channel(ch) if ch.id == ch_id => Some(ch),
                        _ => None,
                    });
                    let ch_name = chan.and_then(|c| c.username.clone());
                    let ch_tittle = chan.map(|c| c.title.clone());
                    (Some(ch_id), ch_name, ch_tittle)
                }
                Some(tl::enums::Peer::Chat(p)) => {
                    let chat_id = p.chat_id;
                    let group = chats.iter().find_map(|g| match g {
                        tl::enums::Chat::Chat(chat) if chat.id == chat_id => Some(chat),
                        _ => None,
                    });
                    let group_name = group.map(|x| x.title.clone());
                    (Some(chat_id), None, group_name)
                }
                None => (
                    fallback_sender.0,
                    fallback_sender.1.clone(),
                    fallback_sender.2.clone(),
                ),
            };

            let tg_comment = TgMessageOutputItem {
                message_id: msg.id,
                sender_id,
                sender_username,
                sender_full_name,
                text: msg.message.clone(),
                reply_to_message_id: msg.reply_to.as_ref().and_then(|r| match r {
                    tl_types::enums::MessageReplyHeader::Header(h) => h.reply_to_msg_id,
                    _ => None,
                }),
            };
            result.push(tg_comment);
        }
        result.reverse();
        Ok(result)
    }

    async fn to_peer_output(&self, peer: &Peer) -> TgPeerOutput {
        match peer {
            Peer::User(u) => {
                return TgPeerOutput::User {
                    id: u.id().bare_id(),
                    full_name: u.full_name(),
                    username: u.username().map(|x| x.to_string()),
                    is_bot: u.is_bot(),
                    is_premium: u.is_premium(),
                    phone_number: u.phone().map(|x| x.to_string()),
                };
            }
            Peer::Group(g) => {
                return TgPeerOutput::Group {
                    id: g.id().bare_id(),
                    title: g.title().map(|x| x.to_string()),
                    username: g.username().map(|x| x.to_string()),
                };
            }
            Peer::Channel(c) => {
                return TgPeerOutput::Channel {
                    id: c.id().bare_id(),
                    tittle: c.title().to_string(),
                    username: c.username().map(|x| x.to_string()),
                };
            }
        }
    }

    async fn to_message_struct(
        &self,
        message: &Message,
        peer: &Peer,
    ) -> Result<TgMessageOutputItem> {
        let m_id = message.id();
        let msg = message.text().to_string();
        let reply = message.reply_to_message_id();
        match peer {
            Peer::User(u) => {
                let username = u.username().map(|x| x.to_string());
                let full_name = u.full_name();
                let m_item = TgMessageOutputItem {
                    message_id: m_id,
                    sender_id: Some(u.id().bare_id()),
                    sender_username: username,
                    sender_full_name: Some(full_name),
                    text: msg,
                    reply_to_message_id: reply,
                };
                return Ok(m_item);
            }
            Peer::Group(g) => {
                let username = g.username().map(|x| x.to_string());
                let full_name = g.title().map(|x| x.to_string());
                let m_item = TgMessageOutputItem {
                    message_id: m_id,
                    sender_id: Some(g.id().bare_id()),
                    sender_username: username,
                    sender_full_name: full_name,
                    text: msg,
                    reply_to_message_id: reply,
                };
                return Ok(m_item);
            }
            Peer::Channel(c) => {
                let username = c.username().map(|x| x.to_string());
                let full_name = c.title().to_string();
                let m_item = TgMessageOutputItem {
                    message_id: m_id,
                    sender_id: Some(c.id().bare_id()),
                    sender_username: username,
                    sender_full_name: Some(full_name),
                    text: msg,
                    reply_to_message_id: reply,
                };
                return Ok(m_item);
            }
        }
    }

    /*pub async fn dbg_func(&self, peer_id: i64, cnt_from: usize, cnt_to: usize) -> Result<()> {
        let peer = self.get_peer_by_id(peer_id).await?;
        let client = self.client.lock().await;
        let mut messages = client.iter_messages(peer.to_ref().await.unwrap());
        let total = messages.total().await.unwrap();
        let to_skip = total - cnt_to + 1;
        let to_save = cnt_to - cnt_from + 1;
        println!("Total {} / To Save {}", total, to_save);
        let mut skip_cnt: usize = 0;
        let mut save_cnt: usize = 0;

        while let Some(msg) = messages.next().await? {
            println!("Skip Count {} / Save count {}", skip_cnt, save_cnt);
            skip_cnt += 1;
            if skip_cnt >= to_skip {
                let g = msg.text();
                let s = msg.sender().unwrap();
                println!(
                    "{} - {} - {} -{}",
                    s.id().bare_id(),
                    s.username().unwrap(),
                    s.name().unwrap(),
                    g
                );
                save_cnt += 1;
            }
            if save_cnt >= to_save {
                break;
            }
        }
        Ok(())
    }*/
}

fn prompt(message: &str) -> Result<String> {
    print!("{message}");
    io::stdout().flush()?;
    let mut line: String = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}
