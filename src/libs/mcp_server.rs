use crate::libs::tg_client::TgClient;
use crate::libs::tg_structs::{GetMessagesRequest, GetPeerRequest};
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{
    ErrorData as McpError,
    handler::server::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde_json::json;
use std::sync::Arc;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};

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
        description = "Counts messages in the Telegram history for a specified peer, which can be a user, group, or channel."
    )]
    async fn get_messages_count_for_peer(
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
        description = "Returns text messages from the specified Telegram peer. The peer can be resolved by username or id according to the request fields. On success, returns JSON in the form {\"text_messages\": [...]}, where each item contains message_id, sender_id, sender_username, sender_full_name, and text."
    )]
    async fn get_text_messages_for_peer(
        &self,
        Parameters(req): Parameters<GetMessagesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text_messages = self
            .client
            .get_messages(req.kind, req.username, req.peer_id, req.limit)
            .await;
        match text_messages {
            Ok(t) => Ok(CallToolResult::success(vec![Content::text(
                json!({"text_messages": t}).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
}

#[tool_handler(name = "tg-mcp-server", version = "0.1.0")]
impl ServerHandler for TelegramMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "Telegram API (MTProto) MCP server. Provides access to Telegram client features such as dialogs, messages, and chats. Functionality is partially implemented and will be expanded over time.".into(),
        );
        info
    }
}

pub async fn start_mcp_server_stream(
    t_client: TgClient,
    bind_address: String,
) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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
