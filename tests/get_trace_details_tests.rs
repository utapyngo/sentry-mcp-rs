use sentry_mcp::api_client::{TraceResponse, TraceTransaction};
use sentry_mcp::tools::get_trace_details::{
    collect_operations, format_duration, format_span_tree, format_trace_output,
};
use serde_json::json;
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

fn make_tx(
    op: Option<&str>,
    duration: Option<f64>,
    children: Vec<TraceTransaction>,
) -> TraceTransaction {
    TraceTransaction {
        event_id: "abc123".to_string(),
        project_id: 1,
        project_slug: "test-project".to_string(),
        transaction: "test-transaction".to_string(),
        start_timestamp: 0.0,
        sdk_name: None,
        timestamp: 1.0,
        children,
        errors: vec![],
        span_id: Some("span1".to_string()),
        parent_span_id: None,
        span_op: op.map(|s| s.to_string()),
        span_description: Some("test description".to_string()),
        span_status: Some("ok".to_string()),
        span_duration: duration,
    }
}

#[test]
fn test_collect_operations_single() {
    let tx = make_tx(Some("http"), Some(100.0), vec![]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&tx, &mut ops);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops.get("http"), Some(&(1, 100.0)));
}

#[test]
fn test_collect_operations_no_op() {
    let tx = make_tx(None, Some(50.0), vec![]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&tx, &mut ops);
    assert!(ops.is_empty());
}

#[test]
fn test_collect_operations_with_children() {
    let child1 = make_tx(Some("db"), Some(50.0), vec![]);
    let child2 = make_tx(Some("db"), Some(30.0), vec![]);
    let tx = make_tx(Some("http"), Some(100.0), vec![child1, child2]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&tx, &mut ops);
    assert_eq!(ops.len(), 2);
    assert_eq!(ops.get("http"), Some(&(1, 100.0)));
    assert_eq!(ops.get("db"), Some(&(2, 80.0)));
}

#[test]
fn test_collect_operations_nested() {
    let grandchild = make_tx(Some("cache"), Some(10.0), vec![]);
    let child = make_tx(Some("db"), Some(50.0), vec![grandchild]);
    let tx = make_tx(Some("http"), Some(100.0), vec![child]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&tx, &mut ops);
    assert_eq!(ops.len(), 3);
    assert_eq!(ops.get("http"), Some(&(1, 100.0)));
    assert_eq!(ops.get("db"), Some(&(1, 50.0)));
    assert_eq!(ops.get("cache"), Some(&(1, 10.0)));
}

#[test]
fn test_format_span_tree_simple() {
    let tx = make_tx(Some("http"), Some(100.0), vec![]);
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("http"));
    assert!(output.contains("test description"));
    assert!(output.contains("test-project"));
    assert!(output.contains("✓"));
}

#[test]
fn test_format_span_tree_with_depth() {
    let child = make_tx(Some("db"), Some(50.0), vec![]);
    let tx = make_tx(Some("http"), Some(100.0), vec![child]);
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("http"));
    assert!(output.contains("db"));
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].starts_with("  "));
}

#[test]
fn test_format_span_tree_error_status() {
    let mut tx = make_tx(Some("http"), Some(100.0), vec![]);
    tx.span_status = Some("internal_error".to_string());
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("✗"));
}

#[test]
fn test_format_span_tree_unknown_op() {
    let tx = make_tx(None, Some(100.0), vec![]);
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("unknown"));
}

#[test]
fn test_format_trace_output_empty() {
    let trace = TraceResponse {
        transactions: vec![],
        orphan_errors: vec![],
    };
    let output = format_trace_output("abc123def456", &trace);
    assert!(output.contains("# Trace Details"));
    assert!(output.contains("**Trace ID:** abc123def456"));
    assert!(output.contains("**Transactions:** 0"));
    assert!(output.contains("**Orphan Errors:** 0"));
}

#[test]
fn test_format_trace_output_with_transaction() {
    let tx = make_tx(Some("http.request"), Some(150.0), vec![]);
    let trace = TraceResponse {
        transactions: vec![tx],
        orphan_errors: vec![],
    };
    let output = format_trace_output("trace-id", &trace);
    assert!(output.contains("**Transactions:** 1"));
    assert!(output.contains("## Operation Breakdown"));
    assert!(output.contains("**http.request**"));
    assert!(output.contains("## Span Tree"));
}

#[test]
fn test_format_trace_output_with_orphan_errors() {
    let trace = TraceResponse {
        transactions: vec![],
        orphan_errors: vec![
            json!({"title": "Error 1", "project_slug": "proj-a"}),
            json!({"title": "Error 2", "project_slug": "proj-b"}),
        ],
    };
    let output = format_trace_output("trace-123", &trace);
    assert!(output.contains("## Orphan Errors"));
    assert!(output.contains("1. Error 1 (proj-a)"));
    assert!(output.contains("2. Error 2 (proj-b)"));
}

#[test]
fn test_format_trace_output_duration_calculation() {
    let mut tx1 = make_tx(Some("http"), Some(100.0), vec![]);
    tx1.start_timestamp = 1000.0;
    tx1.timestamp = 1001.0;
    let mut tx2 = make_tx(Some("db"), Some(50.0), vec![]);
    tx2.start_timestamp = 1000.5;
    tx2.timestamp = 1002.0;
    let trace = TraceResponse {
        transactions: vec![tx1, tx2],
        orphan_errors: vec![],
    };
    let output = format_trace_output("trace-id", &trace);
    assert!(output.contains("**Total Duration:**"));
    assert!(output.contains("2.00s"));
}

#[test]
fn test_format_trace_output_orphan_errors_limited_to_five() {
    let errors: Vec<serde_json::Value> = (1..=10)
        .map(|i| json!({"title": format!("Error {}", i), "project_slug": "proj"}))
        .collect();
    let trace = TraceResponse {
        transactions: vec![],
        orphan_errors: errors,
    };
    let output = format_trace_output("trace-id", &trace);
    assert!(output.contains("5. Error 5"));
    assert!(!output.contains("6. Error 6"));
}

#[test]
fn test_format_span_tree_no_duration() {
    let tx = make_tx(Some("http"), None, vec![]);
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("http"));
}

#[test]
fn test_format_span_tree_no_description() {
    let mut tx = make_tx(Some("http"), Some(100.0), vec![]);
    tx.span_description = None;
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("http"));
}

#[test]
fn test_format_span_tree_no_status() {
    let mut tx = make_tx(Some("http"), Some(100.0), vec![]);
    tx.span_status = None;
    let mut output = String::new();
    format_span_tree(&tx, 0, &mut output);
    assert!(output.contains("http"));
    assert!(output.contains("✓")); // defaults to "ok" status
}

#[test]
fn test_collect_operations_no_duration() {
    let tx = make_tx(Some("http"), None, vec![]);
    let mut ops: HashMap<String, (i32, f64)> = HashMap::new();
    collect_operations(&tx, &mut ops);
    assert_eq!(ops.get("http"), Some(&(1, 0.0)));
}

#[test]
fn test_format_trace_output_multiple_same_operations() {
    let tx1 = make_tx(Some("db.query"), Some(50.0), vec![]);
    let tx2 = make_tx(Some("db.query"), Some(30.0), vec![]);
    let tx3 = make_tx(Some("db.query"), Some(20.0), vec![]);
    let trace = TraceResponse {
        transactions: vec![tx1, tx2, tx3],
        orphan_errors: vec![],
    };
    let output = format_trace_output("trace-id", &trace);
    assert!(output.contains("**db.query**"));
    assert!(output.contains("3 occurrences"));
    assert!(output.contains("100.00ms total"));
}

#[test]
fn test_format_span_tree_deep_nesting() {
    let level3 = make_tx(Some("level3"), Some(10.0), vec![]);
    let level2 = make_tx(Some("level2"), Some(20.0), vec![level3]);
    let level1 = make_tx(Some("level1"), Some(30.0), vec![level2]);
    let root = make_tx(Some("root"), Some(100.0), vec![level1]);
    let mut output = String::new();
    format_span_tree(&root, 0, &mut output);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].starts_with("✓"));
    assert!(lines[1].starts_with("  ✓"));
    assert!(lines[2].starts_with("    ✓"));
    assert!(lines[3].starts_with("      ✓"));
}

#[test]
fn test_format_trace_output_orphan_error_without_title() {
    let errors = vec![json!({"project_slug": "proj"})];
    let trace = TraceResponse {
        transactions: vec![],
        orphan_errors: errors,
    };
    let output = format_trace_output("trace-id", &trace);
    assert!(output.contains("## Orphan Errors"));
    assert!(output.contains("1. Unknown"));
}

#[test]
fn test_format_trace_output_orphan_error_without_project() {
    let errors = vec![json!({"title": "Error"})];
    let trace = TraceResponse {
        transactions: vec![],
        orphan_errors: errors,
    };
    let output = format_trace_output("trace-id", &trace);
    assert!(output.contains("## Orphan Errors"));
    assert!(output.contains("1. Error"));
}
