use sentry_rs::api_client::TraceTransaction;
use sentry_rs::tools::get_trace_details::{collect_operations, format_duration, format_span_tree};
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
