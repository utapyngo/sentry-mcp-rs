use sentry_mcp::api_client::TraceSpan;
use sentry_mcp::tools::get_trace_details::{
    collect_operations, format_duration, format_span_tree, format_trace_output,
    select_interesting_spans,
};
use std::collections::HashMap;

#[test]
fn test_format_duration_milliseconds() {
    assert_eq!(format_duration(100.0), "100.00ms");
    assert_eq!(format_duration(0.5), "0.50ms");
    assert_eq!(format_duration(999.99), "999.99ms");
}

#[test]
fn test_format_duration_seconds() {
    assert_eq!(format_duration(1000.0), "1.00s");
    assert_eq!(format_duration(2500.0), "2.50s");
    assert_eq!(format_duration(10000.0), "10.00s");
}

fn make_span(op: Option<&str>, duration: f64, children: Vec<TraceSpan>) -> TraceSpan {
    TraceSpan {
        event_id: "abc123".to_string(),
        transaction_id: None,
        project_id: 1,
        project_slug: "test-project".to_string(),
        profile_id: None,
        profiler_id: None,
        parent_span_id: None,
        start_timestamp: 0.0,
        end_timestamp: duration / 1000.0,
        duration,
        transaction: Some("test-transaction".to_string()),
        is_transaction: true,
        description: Some("test description".to_string()),
        sdk_name: None,
        op: op.map(|s| s.to_string()),
        name: None,
        children,
        errors: vec![],
        occurrences: vec![],
    }
}

#[test]
fn test_collect_operations_single() {
    let span = make_span(Some("http"), 100.0, vec![]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&span, &mut ops);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops.get("http"), Some(&(1, 100.0)));
}

#[test]
fn test_collect_operations_no_op() {
    let span = make_span(None, 50.0, vec![]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&span, &mut ops);
    assert!(ops.is_empty());
}

#[test]
fn test_collect_operations_with_children() {
    let child1 = make_span(Some("db"), 50.0, vec![]);
    let child2 = make_span(Some("db"), 30.0, vec![]);
    let span = make_span(Some("http"), 100.0, vec![child1, child2]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&span, &mut ops);
    assert_eq!(ops.len(), 2);
    assert_eq!(ops.get("http"), Some(&(1, 100.0)));
    assert_eq!(ops.get("db"), Some(&(2, 80.0)));
}

#[test]
fn test_collect_operations_nested() {
    let grandchild = make_span(Some("cache"), 10.0, vec![]);
    let child = make_span(Some("db"), 50.0, vec![grandchild]);
    let span = make_span(Some("http"), 100.0, vec![child]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&span, &mut ops);
    assert_eq!(ops.len(), 3);
    assert_eq!(ops.get("http"), Some(&(1, 100.0)));
    assert_eq!(ops.get("db"), Some(&(1, 50.0)));
    assert_eq!(ops.get("cache"), Some(&(1, 10.0)));
}

#[test]
fn test_format_span_tree_simple() {
    let span = make_span(Some("http"), 100.0, vec![]);
    let mut output = String::new();
    format_span_tree(&span, 0, &mut output);
    assert!(output.contains("http"));
    assert!(output.contains("test description"));
    assert!(output.contains("test-project"));
    assert!(output.contains("✓"));
}

#[test]
fn test_format_span_tree_with_depth() {
    let child = make_span(Some("db"), 50.0, vec![]);
    let span = make_span(Some("http"), 100.0, vec![child]);
    let mut output = String::new();
    format_span_tree(&span, 0, &mut output);
    assert!(output.contains("http"));
    assert!(output.contains("db"));
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].starts_with("  "));
}

#[test]
fn test_format_span_tree_unknown_op() {
    let span = make_span(None, 100.0, vec![]);
    let mut output = String::new();
    format_span_tree(&span, 0, &mut output);
    assert!(output.contains("unknown"));
}

#[test]
fn test_format_span_tree_error_status() {
    let mut span = make_span(Some("http"), 100.0, vec![]);
    span.errors = vec![serde_json::json!({"title": "error"})];
    let mut output = String::new();
    format_span_tree(&span, 0, &mut output);
    assert!(output.contains("✗"));
}

#[test]
fn test_format_trace_output_empty() {
    let spans: Vec<TraceSpan> = vec![];
    let output = format_trace_output("abc123def456", &spans, None);
    assert!(output.contains("# Trace Details"));
    assert!(output.contains("**Trace ID:** abc123def456"));
    assert!(output.contains("**Transactions:** 0"));
}

#[test]
fn test_format_trace_output_with_transaction() {
    let span = make_span(Some("http.request"), 150.0, vec![]);
    let spans = vec![span];
    let output = format_trace_output("trace-id", &spans, None);
    assert!(output.contains("**Transactions:** 1"));
    assert!(output.contains("## Operation Breakdown"));
    assert!(output.contains("**http.request**"));
}

#[test]
fn test_format_trace_output_duration_calculation() {
    let mut span1 = make_span(Some("http"), 100.0, vec![]);
    span1.start_timestamp = 1000.0;
    span1.end_timestamp = 1001.0;
    let mut span2 = make_span(Some("db"), 50.0, vec![]);
    span2.start_timestamp = 1000.5;
    span2.end_timestamp = 1002.0;
    let spans = vec![span1, span2];
    let output = format_trace_output("trace-id", &spans, None);
    assert!(output.contains("**Total Duration:**"));
    assert!(output.contains("2.00s"));
}

#[test]
fn test_format_trace_output_multiple_same_operations() {
    let span1 = make_span(Some("db.query"), 50.0, vec![]);
    let span2 = make_span(Some("db.query"), 30.0, vec![]);
    let span3 = make_span(Some("db.query"), 20.0, vec![]);
    let spans = vec![span1, span2, span3];
    let output = format_trace_output("trace-id", &spans, None);
    assert!(output.contains("**db.query**"));
    assert!(output.contains("3 occurrences"));
    assert!(output.contains("100.00ms total"));
}

#[test]
fn test_format_span_tree_deep_nesting() {
    let level3 = make_span(Some("level3"), 10.0, vec![]);
    let level2 = make_span(Some("level2"), 20.0, vec![level3]);
    let level1 = make_span(Some("level1"), 30.0, vec![level2]);
    let root = make_span(Some("root"), 100.0, vec![level1]);
    let mut output = String::new();
    format_span_tree(&root, 0, &mut output);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].contains("[root]"));
    assert!(lines[1].starts_with("  "));
    assert!(lines[1].contains("[level1]"));
    assert!(lines[2].starts_with("    "));
    assert!(lines[2].contains("[level2]"));
    assert!(lines[3].starts_with("      "));
    assert!(lines[3].contains("[level3]"));
}

#[test]
fn test_format_span_tree_no_description() {
    let mut span = make_span(Some("http"), 100.0, vec![]);
    span.description = None;
    let mut output = String::new();
    format_span_tree(&span, 0, &mut output);
    assert!(output.contains("http"));
    // falls back to transaction name
    assert!(output.contains("test-transaction"));
}

#[test]
fn test_collect_operations_zero_duration() {
    let span = make_span(Some("http"), 0.0, vec![]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&span, &mut ops);
    assert_eq!(ops.get("http"), Some(&(1, 0.0)));
}

#[test]
fn test_select_interesting_spans_filters_small() {
    let mut small_span = make_span(Some("tiny"), 5.0, vec![]);
    small_span.is_transaction = false;
    let big_span = make_span(Some("http"), 100.0, vec![small_span]);
    let result = select_interesting_spans(&[big_span], 20);
    // big_span is interesting (is_transaction + duration >= 10ms)
    // small_span is NOT interesting (not tx, no errors, duration < 10ms)
    assert!(result.iter().all(|s| s.op.as_deref() != Some("tiny")));
}

#[test]
fn test_select_interesting_spans_includes_transactions() {
    let tx_span = make_span(Some("http"), 5.0, vec![]);
    let result = select_interesting_spans(&[tx_span], 20);
    assert!(!result.is_empty());
    assert!(result[0].is_transaction);
}

#[test]
fn test_select_interesting_spans_max_limit() {
    let spans: Vec<TraceSpan> = (0..30)
        .map(|i| make_span(Some("http"), (i as f64) * 10.0 + 10.0, vec![]))
        .collect();
    let result = select_interesting_spans(&spans, 20);
    assert!(result.len() <= 20);
}

#[test]
fn test_format_trace_output_with_meta() {
    let span = make_span(Some("http"), 100.0, vec![]);
    let meta = sentry_mcp::api_client::TraceMeta {
        logs: 0,
        errors: 3,
        performance_issues: 1,
        span_count: 500.0,
        span_count_map: [("db".to_string(), 200.0), ("http".to_string(), 100.0)]
            .into_iter()
            .collect(),
    };
    let output = format_trace_output("trace-id", &[span], Some(&meta));
    assert!(output.contains("**Total Spans:** 500"));
    assert!(output.contains("**Errors:** 3"));
    assert!(output.contains("**Performance Issues:** 1"));
    assert!(output.contains("## Operation Breakdown"));
    assert!(output.contains("**db**: 200"));
}
