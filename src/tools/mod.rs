pub mod get_issue_details;
pub mod get_trace_details;
pub mod search_issue_events;

use crate::api_client::SentryApiClient;
use get_issue_details::{GetIssueDetailsInput, execute as execute_get_issue_details};
use get_trace_details::{GetTraceDetailsInput, execute as execute_get_trace_details};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool_handler, tool_router,
};
use search_issue_events::{SearchIssueEventsInput, execute as execute_search_events};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct SentryTools {
    client: Arc<SentryApiClient>,
    tool_router: ToolRouter<SentryTools>,
}

impl Default for SentryTools {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl SentryTools {
    pub fn new() -> Self {
        Self {
            client: Arc::new(SentryApiClient::new()),
            tool_router: Self::tool_router(),
        }
    }
    #[rmcp::tool(
        description = "Retrieve detailed information about a specific Sentry issue including metadata, tags, and optionally an event. Accepts either an issueUrl OR (organizationSlug + issueId)."
    )]
    async fn get_issue_details(
        &self,
        Parameters(input): Parameters<GetIssueDetailsInput>,
    ) -> Result<CallToolResult, McpError> {
        info!("get_issue_details: {:?}", input);
        execute_get_issue_details(&*self.client, input).await
    }
    #[rmcp::tool(
        description = "Retrieve trace details including span tree and timing information. Useful for analyzing distributed system performance."
    )]
    async fn get_trace_details(
        &self,
        Parameters(input): Parameters<GetTraceDetailsInput>,
    ) -> Result<CallToolResult, McpError> {
        info!("get_trace_details: {:?}", input);
        execute_get_trace_details(&*self.client, input).await
    }
    #[rmcp::tool(
        description = "Search events for a specific issue using a query string. Returns matching events with their details."
    )]
    async fn search_issue_events(
        &self,
        Parameters(input): Parameters<SearchIssueEventsInput>,
    ) -> Result<CallToolResult, McpError> {
        info!("search_issue_events: {:?}", input);
        execute_search_events(&*self.client, input).await
    }
}

#[tool_handler]
impl ServerHandler for SentryTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "sentry-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            ..Default::default()
        }
    }
}
