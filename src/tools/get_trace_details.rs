use crate::api_client::{SentryApi, TraceResponse, TraceTransaction};
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTraceDetailsInput {
    #[schemars(description = "Organization slug")]
    pub organization_slug: String,
    #[schemars(description = "Trace ID (32-character hex string)")]
    pub trace_id: String,
}

pub fn format_duration(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        format!("{:.2}ms", ms)
    }
}

pub fn collect_operations(tx: &TraceTransaction, ops: &mut HashMap<String, (i32, f64)>) {
    if let Some(op) = &tx.span_op {
        let duration = tx.span_duration.unwrap_or(0.0);
        let entry = ops.entry(op.clone()).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += duration;
    }
    for child in &tx.children {
        collect_operations(child, ops);
    }
}

pub fn format_span_tree(tx: &TraceTransaction, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);
    let duration = tx.span_duration.map(format_duration).unwrap_or_default();
    let op = tx.span_op.as_deref().unwrap_or("unknown");
    let desc = tx.span_description.as_deref().unwrap_or(&tx.transaction);
    let status = tx.span_status.as_deref().unwrap_or("ok");
    let status_icon = if status == "ok" || status == "unknown" {
        "✓"
    } else {
        "✗"
    };
    output.push_str(&format!(
        "{}{} [{}] {} ({}) {}\n",
        indent, status_icon, op, desc, duration, tx.project_slug
    ));
    for child in &tx.children {
        format_span_tree(child, depth + 1, output);
    }
}

pub fn format_trace_output(trace_id: &str, trace: &TraceResponse) -> String {
    let mut output = String::new();
    output.push_str("# Trace Details\n\n");
    output.push_str(&format!("**Trace ID:** {}\n", trace_id));
    output.push_str(&format!("**Transactions:** {}\n", trace.transactions.len()));
    output.push_str(&format!(
        "**Orphan Errors:** {}\n",
        trace.orphan_errors.len()
    ));
    if let Some(root) = trace.transactions.first() {
        let start = root.start_timestamp;
        let end = trace
            .transactions
            .iter()
            .map(|t| t.timestamp)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(root.timestamp);
        let total_duration_ms = (end - start) * 1000.0;
        output.push_str(&format!(
            "**Total Duration:** {}\n",
            format_duration(total_duration_ms)
        ));
    }
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    for tx in &trace.transactions {
        collect_operations(tx, &mut ops);
    }
    if !ops.is_empty() {
        output.push_str("\n## Operation Breakdown\n\n");
        let mut ops_vec: Vec<_> = ops.into_iter().collect();
        ops_vec.sort_by(|a, b| b.1.1.partial_cmp(&a.1.1).unwrap());
        for (op, (count, total_ms)) in ops_vec {
            output.push_str(&format!(
                "- **{}**: {} occurrences, {} total\n",
                op,
                count,
                format_duration(total_ms)
            ));
        }
    }
    output.push_str("\n## Span Tree\n\n```\n");
    for tx in &trace.transactions {
        format_span_tree(tx, 0, &mut output);
    }
    output.push_str("```\n");
    if !trace.orphan_errors.is_empty() {
        output.push_str("\n## Orphan Errors\n\n");
        for (i, err) in trace.orphan_errors.iter().take(5).enumerate() {
            let title = err
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            let project = err
                .get("project_slug")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            output.push_str(&format!("{}. {} ({})\n", i + 1, title, project));
        }
    }
    output
}

pub async fn execute(
    client: &impl SentryApi,
    input: GetTraceDetailsInput,
) -> Result<CallToolResult, McpError> {
    let trace = client
        .get_trace(&input.organization_slug, &input.trace_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let output = format_trace_output(&input.trace_id, &trace);
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        output,
    )]))
}
