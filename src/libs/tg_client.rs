use crate::libs::tg_structs::{
    TgDialogOutputItem, TgMessageOutputItem, TgParticipantOutputItem, TgPeerOutput,
};
use anyhow::Result;
use grammers_client::{
    Client, SenderPool, SignInError,
    peer::{Peer, Role},
};
use grammers_session::storages::SqliteSession;
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

    pub async fn get_peer_info(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<TgPeerOutput> {
        let my_peer = self.get_peer(kind, username, id).await?;
        match my_peer {
            Peer::User(u) => {
                return Ok(TgPeerOutput::User {
                    id: u.id().bare_id(),
                    full_name: u.full_name(),
                    username: u.username().map(|x| x.to_string()),
                    is_bot: u.is_bot(),
                    is_premium: u.is_premium(),
                    phone_number: u.phone().map(|x| x.to_string()),
                });
            }
            Peer::Group(g) => {
                return Ok(TgPeerOutput::Group {
                    id: g.id().bare_id(),
                    title: g.title().map(|x| x.to_string()),
                    username: g.username().map(|x| x.to_string()),
                });
            }
            Peer::Channel(c) => {
                return Ok(TgPeerOutput::Channel {
                    id: c.id().bare_id(),
                    tittle: c.title().to_string(),
                    username: c.username().map(|x| x.to_string()),
                });
            }
        }
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
            .ok_or(format!("no peer with {} username", peer_name))
            .unwrap();
        Ok(peer.clone())
    }

    pub async fn get_messages_count(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<usize> {
        let my_peer = self.get_peer(kind, username, id).await?;
        let client = self.client.lock().await;
        let mut messages = client.iter_messages(my_peer.to_ref().await.unwrap());
        let messages_count = messages.total().await.unwrap();
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
        let client = self.client.lock().await;
        let mut search_messages = client
            .search_messages(my_peer.to_ref().await.unwrap())
            .query(query.as_str());
        let search_count = search_messages.total().await.unwrap();
        Ok(search_count)
    }

    pub async fn get_participants_count(
        &self,
        kind: String,
        username: Option<String>,
        id: Option<i64>,
    ) -> Result<usize> {
        let my_peer = self.get_peer(kind, username, id).await?;
        let client = self.client.lock().await;
        let mut participants = client.iter_participants(my_peer.to_ref().await.unwrap());
        let participants_count = participants.total().await.unwrap();
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
        let client = self.client.lock().await;
        let mut messages = client.iter_messages(peer.to_ref().await.unwrap());
        let total = messages.total().await.unwrap();
        let limit = limit.min(total);
        let mut cnt = 0;
        while let Some(msg) = messages.next().await? {
            let msg_id = msg.id();
            let msg_text = msg.text();
            match msg.sender() {
                Some(sender_peer) => {
                    let m = self
                        .to_message_struct(msg_id, msg_text, sender_peer)
                        .await
                        .unwrap();
                    result.push(m);
                }
                None => {
                    let m = self
                        .to_message_struct(msg_id, msg_text, &peer)
                        .await
                        .unwrap();
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
        let client = self.client.lock().await;
        let mut search_messages = client
            .search_messages(peer.to_ref().await.unwrap())
            .query(query.as_str());
        let mut cnt = 0;
        while let Some(msg) = search_messages.next().await? {
            let msg_id = msg.id();
            let msg_text = msg.text();
            match msg.sender() {
                Some(sender_peer) => {
                    let m = self
                        .to_message_struct(msg_id, msg_text, sender_peer)
                        .await
                        .unwrap();
                    result.push(m);
                }
                None => {
                    let m = self
                        .to_message_struct(msg_id, msg_text, &peer)
                        .await
                        .unwrap();
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
        let client = self.client.lock().await;
        let mut participants = client.iter_participants(peer.to_ref().await.unwrap());
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
        message: String,
    ) -> Result<()> {
        let peer = self.get_peer(kind, username, id).await?;
        let client = self.client.lock().await;
        client
            .send_message(peer.to_ref().await.unwrap(), message.as_str())
            .await?;
        Ok(())
    }

    async fn to_message_struct(
        &self,
        m_id: i32,
        msg: &str,
        peer: &Peer,
    ) -> Result<TgMessageOutputItem> {
        match peer {
            Peer::User(u) => {
                let username = u.username().map(|x| x.to_string());
                let full_name = u.full_name();
                let m_item = TgMessageOutputItem {
                    message_id: m_id,
                    sender_id: u.id().bare_id(),
                    sender_username: username,
                    sender_full_name: Some(full_name),
                    text: msg.to_string(),
                };
                return Ok(m_item);
            }
            Peer::Group(g) => {
                let username = g.username().map(|x| x.to_string());
                let full_name = g.title().map(|x| x.to_string());
                let m_item = TgMessageOutputItem {
                    message_id: m_id,
                    sender_id: g.id().bare_id(),
                    sender_username: username,
                    sender_full_name: full_name,
                    text: msg.to_string(),
                };
                return Ok(m_item);
            }
            Peer::Channel(c) => {
                let username = c.username().map(|x| x.to_string());
                let full_name = c.title().to_string();
                let m_item = TgMessageOutputItem {
                    message_id: m_id,
                    sender_id: c.id().bare_id(),
                    sender_username: username,
                    sender_full_name: Some(full_name),
                    text: msg.to_string(),
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
