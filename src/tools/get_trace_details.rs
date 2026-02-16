use crate::api_client::{SentryApi, TraceMeta, TraceSpan};
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;

/// Minimum span duration in ms to be considered interesting.
const MIN_INTERESTING_DURATION_MS: f64 = 10.0;
/// Maximum number of interesting spans to display.
const MAX_INTERESTING_SPANS: usize = 20;
/// A span is "dominated" if its single child takes this fraction of its duration.
const DOMINATED_THRESHOLD: f64 = 0.9;

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

pub fn collect_operations(span: &TraceSpan, ops: &mut HashMap<String, (i32, f64)>) {
    if let Some(op) = &span.op {
        let entry = ops.entry(op.clone()).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += span.duration;
    }
    for child in &span.children {
        collect_operations(child, ops);
    }
}

pub fn format_span_tree(span: &TraceSpan, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);
    let duration = format_duration(span.duration);
    let op = span.op.as_deref().unwrap_or("unknown");
    let desc = span
        .description
        .as_deref()
        .or(span.transaction.as_deref())
        .unwrap_or("(no description)");
    let has_errors = !span.errors.is_empty();
    let status_icon = if has_errors { "✗" } else { "✓" };
    let tx_marker = if span.is_transaction { " [tx]" } else { "" };
    output.push_str(&format!(
        "{}{} [{}] {} ({}) {}{}\n",
        indent, status_icon, op, desc, duration, span.project_slug, tx_marker
    ));
    for child in &span.children {
        format_span_tree(child, depth + 1, output);
    }
}

/// Filter spans to show only interesting ones for display.
/// Always includes transactions, spans with errors, and spans >= MIN_INTERESTING_DURATION_MS.
/// Sorted by duration, truncated to max_spans.
pub fn select_interesting_spans(spans: &[TraceSpan], max_spans: usize) -> Vec<TraceSpan> {
    let mut collected: Vec<TraceSpan> = Vec::new();
    for span in spans {
        collect_interesting(span, &mut collected);
    }
    collected.sort_by(|a, b| b.duration.partial_cmp(&a.duration).unwrap());
    collected.truncate(max_spans);
    collected
}

fn collect_interesting(span: &TraceSpan, out: &mut Vec<TraceSpan>) {
    let dominated_by_one_child = span.children.len() == 1
        && span.children[0].duration >= span.duration * DOMINATED_THRESHOLD;

    // Skip non-transaction spans that are dominated by a single child
    // (e.g., middleware chains where each middleware wraps the next)
    let dominated_skip = dominated_by_one_child && !span.is_transaction;

    let is_interesting = span.is_transaction || !span.errors.is_empty() || span.duration >= MIN_INTERESTING_DURATION_MS;

    if !dominated_skip && is_interesting {
        let mut filtered = span.clone();
        filtered.children = Vec::new();
        out.push(filtered);
    }

    for child in &span.children {
        collect_interesting(child, out);
    }
}

pub fn format_trace_output(
    trace_id: &str,
    spans: &[TraceSpan],
    meta: Option<&TraceMeta>,
) -> String {
    let mut output = String::new();
    output.push_str("# Trace Details\n\n");
    output.push_str(&format!("**Trace ID:** {}\n", trace_id));

    let tx_count = count_transactions(spans);
    output.push_str(&format!("**Transactions:** {}\n", tx_count));

    if let Some(meta) = meta {
        output.push_str(&format!("**Total Spans:** {}\n", meta.span_count as i64));
        output.push_str(&format!("**Errors:** {}\n", meta.errors));
        output.push_str(&format!(
            "**Performance Issues:** {}\n",
            meta.performance_issues
        ));
    }

    if let Some(root) = spans.first() {
        let (start, end) = compute_time_range(spans);
        let total_duration_ms = (end - start) * 1000.0;
        if total_duration_ms > 0.0 {
            output.push_str(&format!(
                "**Total Duration:** {}\n",
                format_duration(total_duration_ms)
            ));
        } else {
            output.push_str(&format!(
                "**Root Duration:** {}\n",
                format_duration(root.duration)
            ));
        }
    }

    if let Some(meta) = meta
        && !meta.span_count_map.is_empty()
    {
        output.push_str("\n## Operation Breakdown\n\n");
        let mut map_vec: Vec<_> = meta.span_count_map.iter().collect();
        map_vec.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (op, count) in map_vec {
            output.push_str(&format!("- **{}**: {}\n", op, *count as i64));
        }
    } else {
        let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
        for span in spans {
            collect_operations(span, &mut ops);
        }
        if !ops.is_empty() {
            output.push_str("\n## Operation Breakdown\n\n");
            let mut ops_vec: Vec<_> = ops.into_iter().collect();
            ops_vec.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap());
            for (op, (count, total_ms)) in ops_vec {
                output.push_str(&format!(
                    "- **{}**: {} occurrences, {} total\n",
                    op,
                    count,
                    format_duration(total_ms)
                ));
            }
        }
    }

    let interesting = select_interesting_spans(spans, MAX_INTERESTING_SPANS);
    output.push_str("\n## Span Tree\n\n```\n");
    for span in &interesting {
        format_span_tree(span, 0, &mut output);
    }
    output.push_str("```\n");

    output
}

fn count_transactions(spans: &[TraceSpan]) -> usize {
    let mut count = 0;
    for span in spans {
        if span.is_transaction {
            count += 1;
        }
        count += count_transactions(&span.children);
    }
    count
}

fn compute_time_range(spans: &[TraceSpan]) -> (f64, f64) {
    let mut min_start = f64::MAX;
    let mut max_end = f64::MIN;
    for span in spans {
        if span.start_timestamp > 0.0 && span.start_timestamp < min_start {
            min_start = span.start_timestamp;
        }
        if span.end_timestamp > 0.0 && span.end_timestamp > max_end {
            max_end = span.end_timestamp;
        }
        let (child_start, child_end) = compute_time_range(&span.children);
        if child_start < min_start {
            min_start = child_start;
        }
        if child_end > max_end {
            max_end = child_end;
        }
    }
    (min_start, max_end)
}

pub async fn execute(
    client: &impl SentryApi,
    input: GetTraceDetailsInput,
) -> Result<CallToolResult, McpError> {
    let trace = client
        .get_trace(&input.organization_slug, &input.trace_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let meta = client
        .get_trace_meta(&input.organization_slug, &input.trace_id)
        .await
        .ok();
    let output = format_trace_output(&input.trace_id, &trace, meta.as_ref());
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        output,
    )]))
}
