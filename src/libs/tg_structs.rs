use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub sender_id: i64,
    pub sender_username: Option<String>,
    pub sender_full_name: Option<String>,
    pub text: String,
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
pub struct GetMessagesRequest {
    #[schemars(description = "How to resolve the Telegram peer: 'username' or 'id'.")]
    pub kind: String,

    #[schemars(
        description = "Telegram public username, with or without leading @. Required when kind='username'."
    )]
    pub username: Option<String>,

    #[schemars(description = "Telegram peer id returned by get_dialogs. Required when kind='id'.")]
    pub peer_id: Option<i64>,

    #[schemars(description = "Maximum number of messages to return.")]
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TgPeerOutput {
    User {
        id: i64,
        full_name: String,
        username: Option<String>,
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
