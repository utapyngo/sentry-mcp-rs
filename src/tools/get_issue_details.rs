use crate::api_client::SentryApiClient;
use regex::Regex;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

fn format_frame_detail(output: &mut String, frame: &Value) {
    let filename = frame.get("filename").and_then(|v| v.as_str()).unwrap_or("?");
    let lineno = frame.get("lineNo").and_then(|v| v.as_i64()).unwrap_or(0);
    let func = frame.get("function").and_then(|v| v.as_str()).unwrap_or("?");
    output.push_str(&format!("─────────────────────\n  File \"{}\", line {}, in {}\n\n", filename, lineno, func));
    if let Some(context) = frame.get("context").and_then(|v| v.as_array()) {
        for line in context {
            if let Some(arr) = line.as_array()
                && arr.len() >= 2
            {
                let num = arr[0].as_i64().unwrap_or(0);
                let code = arr[1].as_str().unwrap_or("");
                let marker = if num == lineno { "  → " } else { "    " };
                output.push_str(&format!("{}{} │{}\n", marker, num, code));
            }
        }
    }
    if let Some(vars) = frame.get("vars").and_then(|v| v.as_object())
        && !vars.is_empty()
    {
        output.push_str("\nLocal Variables:\n");
        for (key, val) in vars {
            let val_str = match val {
                Value::String(s) => format!("\"{}\"", s),
                Value::Null => "None".to_string(),
                _ => val.to_string(),
            };
            let truncated = if val_str.chars().count() > 60 {
                format!("{}...", val_str.chars().take(57).collect::<String>())
            } else {
                val_str
            };
            output.push_str(&format!("├─ {}: {}\n", key, truncated));
        }
    }
}

fn format_exception(output: &mut String, exc: &Value) {
    let exc_type = exc.get("type").and_then(|v| v.as_str()).unwrap_or("Error");
    let exc_value = exc.get("value").and_then(|v| v.as_str()).unwrap_or("");
    output.push_str(&format!("\n### {}: {}\n", exc_type, exc_value));
    if let Some(stacktrace) = exc.get("stacktrace")
        && let Some(frames) = stacktrace.get("frames").and_then(|v| v.as_array())
    {
        let frames_vec: Vec<_> = frames.iter().collect();
        if let Some(relevant) = frames_vec
            .iter()
            .rev()
            .find(|f| f.get("inApp").and_then(|v| v.as_bool()).unwrap_or(false))
        {
            output.push_str("\n**Most Relevant Frame:**\n");
            format_frame_detail(output, relevant);
        }
        output.push_str("\n**Full Stacktrace:**\n────────────────\n```\n");
        for frame in frames_vec.iter().rev().take(20) {
            let filename = frame.get("filename").and_then(|v| v.as_str()).unwrap_or("?");
            let lineno = frame.get("lineNo").and_then(|v| v.as_i64()).unwrap_or(0);
            let func = frame.get("function").and_then(|v| v.as_str()).unwrap_or("?");
            let context_line = frame
                .get("context")
                .and_then(|v| v.as_array())
                .and_then(|ctx| {
                    ctx.iter().find(|line| {
                        line.as_array()
                            .map(|arr| arr.first().and_then(|n| n.as_i64()) == Some(lineno))
                            .unwrap_or(false)
                    })
                })
                .and_then(|line| line.as_array())
                .and_then(|arr| arr.get(1))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            output.push_str(&format!("  File \"{}\", line {}, in {}\n", filename, lineno, func));
            if !context_line.is_empty() {
                output.push_str(&format!("        {}\n", context_line.trim()));
            }
        }
        output.push_str("```\n");
    }
}

fn format_event_entries(output: &mut String, entries: &[crate::api_client::EventEntry]) {
    for entry in entries {
        if entry.entry_type == "exception" {
            if let Some(values) = entry.data.get("values").and_then(|v| v.as_array()) {
                for exc in values {
                    format_exception(output, exc);
                }
            }
        } else if entry.entry_type == "message"
            && let Some(msg) = entry.data.get("formatted").and_then(|v| v.as_str())
        {
            output.push_str(&format!("\n### Message\n{}\n", msg));
        }
    }
}

fn format_extra_data(output: &mut String, extra: &serde_json::Map<String, Value>) {
    output.push_str("\n### Extra Data\n");
    for (key, val) in extra {
        let v_str = match val {
            Value::String(s) => format!("\"{}\"", s),
            Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => format!("\"{}\"", s),
                        _ => v.to_string(),
                    })
                    .collect();
                format!("[{}]", items.join(", "))
            }
            _ => val.to_string(),
        };
        output.push_str(&format!("**{}:** {}\n", key, v_str));
    }
}

fn format_contexts(output: &mut String, contexts: &serde_json::Map<String, Value>) {
    output.push_str("\n### Context\n");
    for (key, val) in contexts {
        if let Some(obj) = val.as_object() {
            output.push_str(&format!("**{}:**\n", key));
            for (k, v) in obj {
                let v_str = match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                output.push_str(&format!("  {}: {}\n", k, v_str));
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIssueDetailsInput {
    #[schemars(description = "Full Sentry issue URL")]
    pub issue_url: Option<String>,
    #[schemars(description = "Organization slug (required if issue_url not provided)")]
    pub organization_slug: Option<String>,
    #[schemars(description = "Issue ID like 'PROJECT-123' or numeric ID (required if issue_url not provided)")]
    pub issue_id: Option<String>,
    #[schemars(description = "Specific event ID to fetch instead of latest")]
    pub event_id: Option<String>,
}

fn parse_issue_url(url: &str) -> Option<(String, String)> {
    let re = Regex::new(r"https?://[^/]+/organizations/([^/]+)/issues/([^/?]+)").ok()?;
    let caps = re.captures(url)?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

pub async fn execute(
    client: &SentryApiClient,
    input: GetIssueDetailsInput,
) -> Result<CallToolResult, McpError> {
    let (org_slug, issue_id) = if let Some(url) = &input.issue_url {
        parse_issue_url(url).ok_or_else(|| {
            McpError::invalid_params("Invalid issue URL format", None)
        })?
    } else {
        let org = input.organization_slug.ok_or_else(|| {
            McpError::invalid_params(
                "Either issue_url or organization_slug + issue_id required",
                None,
            )
        })?;
        let id = input.issue_id.ok_or_else(|| {
            McpError::invalid_params(
                "Either issue_url or organization_slug + issue_id required",
                None,
            )
        })?;
        (org, id)
    };
    let issue = client
        .get_issue(&org_slug, &issue_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let event = if let Some(event_id) = &input.event_id {
        client
            .get_event(&org_slug, &issue_id, event_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
    } else {
        client
            .get_latest_event(&org_slug, &issue_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
    };
    let mut output = String::new();
    output.push_str("# Issue Details\n\n");
    output.push_str(&format!("**ID:** {}\n", issue.short_id));
    output.push_str(&format!("**Title:** {}\n", issue.title));
    output.push_str(&format!("**Status:** {}\n", issue.status));
    if let Some(substatus) = &issue.substatus {
        output.push_str(&format!("**Substatus:** {}\n", substatus));
    }
    if let Some(issue_type) = &issue.issue_type {
        output.push_str(&format!("**Issue Type:** {}\n", issue_type));
    }
    if let Some(issue_category) = &issue.issue_category {
        output.push_str(&format!("**Issue Category:** {}\n", issue_category));
    }
    if let Some(level) = &issue.level {
        output.push_str(&format!("**Level:** {}\n", level));
    }
    if let Some(culprit) = &issue.culprit {
        output.push_str(&format!("**Culprit:** {}\n", culprit));
    }
    output.push_str(&format!("**Project:** {} ({})\n", issue.project.name, issue.project.slug));
    if let Some(platform) = &issue.platform {
        output.push_str(&format!("**Platform:** {}\n", platform));
    }
    output.push_str(&format!("**First Seen:** {}\n", issue.first_seen));
    output.push_str(&format!("**Last Seen:** {}\n", issue.last_seen));
    output.push_str(&format!("**Event Count:** {}\n", issue.count));
    output.push_str(&format!("**User Count:** {}\n", issue.user_count));
    if let Some(permalink) = &issue.permalink {
        output.push_str(&format!("**URL:** {}\n", permalink));
    }
    if !issue.tags.is_empty() {
        output.push_str("\n## Tags\n");
        for tag in &issue.tags {
            output.push_str(&format!("- **{}:** {} ({} events)\n", tag.key, tag.name, tag.total_values));
        }
    }
    output.push_str("\n## Latest Event\n\n");
    output.push_str(&format!("**Event ID:** {}\n", event.event_id));
    output.push_str(&format!("**Date:** {}\n", event.date_created));
    if let Some(msg) = &event.message {
        output.push_str(&format!("**Message:** {}\n", msg));
    }
    format_event_entries(&mut output, &event.entries);
    if !event.tags.is_empty() {
        output.push_str("\n### Event Tags\n");
        for tag in &event.tags {
            output.push_str(&format!("**{}:** {}\n", tag.key, tag.value));
        }
    }
    if let Some(extra) = event.context.as_object()
        && !extra.is_empty()
    {
        format_extra_data(&mut output, extra);
    }
    if let Some(contexts) = event.contexts.as_object()
        && !contexts.is_empty()
    {
        format_contexts(&mut output, contexts);
    }
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(output)]))
}
