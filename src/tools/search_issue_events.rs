use crate::api_client::{Event, EventsQuery, SentryApi};
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchIssueEventsInput {
    #[schemars(description = "Organization slug")]
    pub organization_slug: String,
    #[schemars(description = "Issue ID like 'PROJECT-123' or numeric ID")]
    pub issue_id: String,
    #[schemars(
        description = "Sentry search query. Syntax: key:value pairs with optional raw text. \
        Operators: > < >= <= for numbers, ! for negation, * for wildcard, OR/AND for logic. \
        Event properties: environment, release, platform, message, user.id, user.email, \
        device.family, browser.name, os.name, server_name, transaction. \
        Examples: 'server_name:web-1', 'environment:production', '!user.email:*@test.com', \
        'browser.name:Chrome OR browser.name:Firefox'"
    )]
    pub query: Option<String>,
    #[schemars(description = "Maximum number of events to return (default: 10, max: 100)")]
    pub limit: Option<i32>,
    #[schemars(description = "Sort order: 'newest' (default) or 'oldest'")]
    pub sort: Option<String>,
}

pub fn format_events_output(issue_id: &str, query: Option<&str>, events: &[Event]) -> String {
    let mut output = String::new();
    output.push_str("# Issue Events\n\n");
    output.push_str(&format!("**Issue:** {}\n", issue_id));
    if let Some(q) = query {
        output.push_str(&format!("**Query:** {}\n", q));
    }
    output.push_str(&format!("**Found:** {} events\n\n", events.len()));
    for (i, event) in events.iter().enumerate() {
        output.push_str(&format!("## Event {} - {}\n\n", i + 1, event.event_id));
        if let Some(date) = &event.date_created {
            output.push_str(&format!("**Date:** {}\n", date));
        }
        if let Some(platform) = &event.platform {
            output.push_str(&format!("**Platform:** {}\n", platform));
        }
        if let Some(msg) = &event.message
            && !msg.is_empty()
        {
            output.push_str(&format!("**Message:** {}\n", msg));
        }
        if !event.tags.is_empty() {
            output.push_str("**Tags:** ");
            let tags: Vec<String> = event
                .tags
                .iter()
                .map(|t| format!("{}={}", t.key, t.value))
                .collect();
            output.push_str(&tags.join(", "));
            output.push('\n');
        }
        for entry in &event.entries {
            if entry.entry_type == "exception"
                && let Some(values) = entry.data.get("values").and_then(|v| v.as_array())
            {
                for exc in values {
                    let exc_type = exc.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                    let exc_value = exc.get("value").and_then(|v| v.as_str()).unwrap_or("?");
                    output.push_str(&format!("**Exception:** {} - {}\n", exc_type, exc_value));
                }
            }
        }
        output.push('\n');
    }
    if events.is_empty() {
        output.push_str("No events found matching the query.\n");
    }
    output
}

pub async fn execute(
    client: &impl SentryApi,
    input: SearchIssueEventsInput,
) -> Result<CallToolResult, McpError> {
    let limit = input.limit.unwrap_or(10).min(100);
    let sort = input.sort.unwrap_or_else(|| "newest".to_string());
    let query = EventsQuery {
        query: input.query.clone(),
        limit: Some(limit),
        sort: Some(sort),
    };
    let events = client
        .list_events_for_issue(&input.organization_slug, &input.issue_id, &query)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let output = format_events_output(&input.issue_id, input.query.as_deref(), &events);
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        output,
    )]))
}
