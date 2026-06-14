use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_limit() -> usize {
    50
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TgDialogOutputItem {
    pub dialog_id: i64,
    pub dialog_name: Option<String>,
    pub dialog_full_name: Option<String>,
    pub dialog_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TgMessageOutputItem {
    pub message_id: i32,
    pub sender_id: Option<i64>,
    pub sender_username: Option<String>,
    pub sender_full_name: Option<String>,
    pub reply_to_message_id: Option<i32>,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TgParticipantOutputItem {
    pub id: i64,
    pub full_name: String,
    pub username: Option<String>,
    pub is_bot: bool,
    pub is_premium: bool,
    pub phone_number: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetPeerRequest {
    #[schemars(description = "How to resolve the Telegram peer: username or id.")]
    pub kind: String,

    #[schemars(description = "Telegram public username, without @. Required when kind=username.")]
    pub username: Option<String>,

    #[schemars(description = "Telegram peer id returned by get_dialogs. Required when kind=id.")]
    pub id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SearchPeerRequest {
    #[schemars(description = "Search query used to find Telegram users, groups, or channels.")]
    pub query: String,

    #[schemars(description = "Maximum number of matching Telegram peers to return.")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PeerDataRequest {
    #[schemars(
        description = "How to resolve the Telegram peer. Must be either 'id' or 'username'."
    )]
    pub peer_kind: String,

    #[schemars(
        description = "Telegram username, with or without leading @. Required when peer_kind is 'username'."
    )]
    pub peer_username: Option<String>,

    #[schemars(description = "Telegram peer id. Required when peer_kind is 'id'.")]
    pub peer_id: Option<i64>,

    #[schemars(description = "Maximum number of items to return.")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetPostCommentsRequest {
    #[schemars(
        description = "How to resolve the Telegram peer. Must be either 'id' or 'username'."
    )]
    pub peer_kind: String,

    #[schemars(
        description = "Telegram username, with or without leading @. Required when peer_kind is 'username'."
    )]
    pub peer_username: Option<String>,

    #[schemars(description = "Telegram peer id. Required when peer_kind is 'id'.")]
    pub peer_id: Option<i64>,

    #[schemars(description = "Identifier of the channel post whose comments should be retrieved.")]
    pub message_id: i32,

    #[schemars(description = "Maximum number of comments to return for the specified post.")]
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetSearchMessagesRequest {
    #[schemars(
        description = "How to resolve the Telegram peer. Must be either 'id' or 'username'."
    )]
    pub peer_kind: String,

    #[schemars(
        description = "Telegram username, with or without leading @. Required when peer_kind is 'username'."
    )]
    pub peer_username: Option<String>,

    #[schemars(description = "Telegram peer id. Required when peer_kind is 'id'.")]
    pub peer_id: Option<i64>,

    #[schemars(
        description = "Search query text used to find matching messages in the selected Telegram peer."
    )]
    pub query: String,

    #[serde(default = "default_limit")]
    #[schemars(description = "Maximum number of matching messages to return. Defaults to 50.")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SendMessageRequest {
    #[schemars(
        description = "How to resolve the Telegram peer. Must be either 'id' or 'username'."
    )]
    pub peer_kind: String,

    #[schemars(
        description = "Telegram username, with or without leading @. Required when peer_kind is 'username'."
    )]
    pub peer_username: Option<String>,

    #[schemars(description = "Telegram peer id. Required when peer_kind is 'id'.")]
    pub peer_id: Option<i64>,

    #[schemars(description = "Text message content to send to the selected Telegram peer.")]
    pub message: String,

    #[schemars(
        description = "Optional message id to reply to. If set, the message will be sent as a reply to the specified message."
    )]
    pub reply_to_message_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SendPostComment {
    #[schemars(
        description = "How to resolve the Telegram peer. Must be either 'id' or 'username'."
    )]
    pub peer_kind: String,

    #[schemars(
        description = "Telegram username, with or without leading @. Required when peer_kind is 'username'."
    )]
    pub peer_username: Option<String>,

    #[schemars(description = "Telegram peer id. Required when peer_kind is 'id'.")]
    pub peer_id: Option<i64>,

    #[schemars(description = "Identifier of the channel post to comment on.")]
    pub message_id: i32,

    #[schemars(
        description = "Text content of the comment to be posted in the discussion thread associated with the specified channel post."
    )]
    pub post_comment: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TgPeerOutput {
    User {
        id: i64,
        full_name: String,
        username: Option<String>,
        is_bot: bool,
        is_premium: bool,
        phone_number: Option<String>,
    },
    Group {
        id: i64,
        title: Option<String>,
        username: Option<String>,
    },
    Channel {
        id: i64,
        tittle: String,
        username: Option<String>,
    },
}
