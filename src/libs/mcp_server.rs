use crate::libs::tg_client::TgClient;
use crate::libs::tg_structs::{
    GetPeerRequest, GetSearchMessagesRequest, PeerLimitRequest, SearchPeerRequest,
    SendMessageRequest,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct TelegramMcpServer {
    client: Arc<TgClient>,
    tool_router: ToolRouter<TelegramMcpServer>,
}

#[tool_router]
impl TelegramMcpServer {
    pub fn new(t_client: Arc<TgClient>) -> Self {
        Self {
            client: t_client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Health check: returns service status to confirm the MCP server is running."
    )]
    async fn is_alive(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "Telegram MCP is running".to_string(),
        )]))
    }

    #[tool(
        description = "Checks whether the Telegram client has an active authorized session and is ready to access Telegram data."
    )]
    async fn is_authorized(&self) -> Result<CallToolResult, McpError> {
        let is_auth = self.client.is_authorized().await.unwrap_or(false);
        Ok(CallToolResult::success(vec![Content::text(
            json!({"is_authorized": is_auth}).to_string(),
        )]))
    }

    #[tool(
        description = "Returns the list of available Telegram dialogs, including private chats, groups, and channels accessible to the current client session"
    )]
    async fn get_all_dialogs(&self) -> Result<CallToolResult, McpError> {
        let dialogs = self.client.get_dialogs().await;
        match dialogs {
            Ok(d) => Ok(CallToolResult::success(vec![Content::text(
                json!({"dialogs": d}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Returns information about a Telegram peer by username or id. The peer can be a user, group, or channel."
    )]
    async fn get_peer(
        &self,
        Parameters(req): Parameters<GetPeerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let peer_data = self
            .client
            .get_peer_info(req.kind, req.username, req.id)
            .await;
        match peer_data {
            Ok(p) => Ok(CallToolResult::success(vec![Content::text(
                json!({"peer": p}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Searches for Telegram peers matching the provided query and returns users, groups, and channels that satisfy the search expression."
    )]
    async fn search_peer(
        &self,
        Parameters(req): Parameters<SearchPeerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let found_peers = self.client.search_peer(req.query, req.limit).await;
        match found_peers {
            Ok(p) => Ok(CallToolResult::success(vec![Content::text(
                json!({"found_peers": p}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Counts messages in the Telegram history for a specified peer, which can be a user, group, or channel."
    )]
    async fn get_text_messages_count(
        &self,
        Parameters(req): Parameters<GetPeerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let total_messages_count = self
            .client
            .get_messages_count(req.kind, req.username, req.id)
            .await;
        match total_messages_count {
            Ok(cnt) => Ok(CallToolResult::success(vec![Content::text(
                json!({"total_messages_count": cnt}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Counts participants in the specified Telegram peer. The peer should be a group or channel; private users do not have participants."
    )]
    async fn get_participants_count(
        &self,
        Parameters(req): Parameters<GetPeerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let total_participants_count = self
            .client
            .get_participants_count(req.kind, req.username, req.id)
            .await;
        match total_participants_count {
            Ok(cnt) => Ok(CallToolResult::success(vec![Content::text(
                json!({"total_participants_count": cnt}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Returns the number of Telegram text messages from the specified peer that match the provided search query."
    )]
    async fn search_text_messages_count(
        &self,
        Parameters(req): Parameters<GetSearchMessagesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let total_search_count = self
            .client
            .get_search_messages_count(req.peer.kind, req.peer.username, req.peer.id, req.query)
            .await;
        match total_search_count {
            Ok(cnt) => Ok(CallToolResult::success(vec![Content::text(
                json!({"total_search_count": cnt}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Returns text messages from the specified Telegram peer, resolved by username or id. The response contains a `text_messages` array with message id, sender information, and message text."
    )]
    async fn get_text_messages(
        &self,
        Parameters(req): Parameters<PeerLimitRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text_messages = self
            .client
            .get_messages(req.peer.kind, req.peer.username, req.peer.id, req.limit)
            .await;
        match text_messages {
            Ok(t) => Ok(CallToolResult::success(vec![Content::text(
                json!({"text_messages": t}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Searches text messages in the specified Telegram peer and returns only messages that satisfy the provided query expression."
    )]
    async fn search_text_messages(
        &self,
        Parameters(req): Parameters<GetSearchMessagesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let search_messages = self
            .client
            .get_search_messages(
                req.peer.kind,
                req.peer.username,
                req.peer.id,
                req.query,
                req.limit,
            )
            .await;
        match search_messages {
            Ok(t) => Ok(CallToolResult::success(vec![Content::text(
                json!({"text_messages": t}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Fetches participants for the specified Telegram peer. The peer should be a group or channel; private user chats do not have participants."
    )]
    async fn get_participants(
        &self,
        Parameters(req): Parameters<PeerLimitRequest>,
    ) -> Result<CallToolResult, McpError> {
        let participants = self
            .client
            .get_participants(req.peer.kind, req.peer.username, req.peer.id, req.limit)
            .await;
        match participants {
            Ok(p) => Ok(CallToolResult::success(vec![Content::text(
                json!({"participants": p}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Sends a text message to the specified Telegram peer. The peer can be a user, group, or channel resolved by username or id."
    )]
    async fn send_message(
        &self,
        Parameters(req): Parameters<SendMessageRequest>,
    ) -> Result<CallToolResult, McpError> {
        let send_status = self
            .client
            .send_message(req.peer.kind, req.peer.username, req.peer.id, req.message)
            .await;
        match send_status {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                json!({"send_status": true}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
}

#[tool_handler]
impl ServerHandler for TelegramMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info.name = env!("CARGO_PKG_NAME").to_string();
        info.server_info.version = env!("CARGO_PKG_VERSION").to_string();
        info.instructions = Some(
            "Telegram API (MTProto) MCP server. Provides access to Telegram client features such as dialogs, messages, and chats. Functionality is partially implemented and will be expanded over time.".into(),
        );
        info
    }
}

pub async fn start_mcp_server_http(t_client: TgClient, bind_address: String) -> anyhow::Result<()> {
    let tg = Arc::new(t_client);
    let ct = tokio_util::sync::CancellationToken::new();
    let service = StreamableHttpService::new(
        {
            let tg = Arc::clone(&tg);
            move || Ok(TelegramMcpServer::new(Arc::clone(&tg)))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(bind_address).await?;
    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.unwrap();
            ct.cancel();
        })
        .await;
    Ok(())
}

pub async fn start_mcp_server_stdio(t_client: TgClient) -> anyhow::Result<()> {
    let tg = Arc::new(t_client);
    let server = TelegramMcpServer::new(Arc::clone(&tg));
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
