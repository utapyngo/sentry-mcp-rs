use sentry_mcp::api_client::{Event, EventEntry, EventTag};
use sentry_mcp::tools::search_issue_events::format_events_output;
use serde_json::json;

fn make_event(
    event_id: &str,
    date: &str,
    platform: Option<&str>,
    message: Option<&str>,
    tags: Vec<(&str, &str)>,
    entries: Vec<EventEntry>,
) -> Event {
    Event {
        id: "id".to_string(),
        event_id: event_id.to_string(),
        date_created: Some(date.to_string()),
        message: message.map(|s| s.to_string()),
        platform: platform.map(|s| s.to_string()),
        entries,
        contexts: json!({}),
        context: json!({}),
        tags: tags
            .into_iter()
            .map(|(k, v)| EventTag {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
    }
}

#[test]
fn test_format_events_empty() {
    let output = format_events_output("PROJ-123", None, &[]);
    assert!(output.contains("# Issue Events"));
    assert!(output.contains("**Issue:** PROJ-123"));
    assert!(output.contains("**Found:** 0 events"));
    assert!(output.contains("No events found matching the query."));
}

#[test]
fn test_format_events_with_query() {
    let output = format_events_output("PROJ-123", Some("environment:prod"), &[]);
    assert!(output.contains("**Query:** environment:prod"));
}

#[test]
fn test_format_events_single_event() {
    let events = vec![make_event(
        "abc123",
        "2024-01-15T10:00:00Z",
        Some("python"),
        Some("Error occurred"),
        vec![("env", "prod")],
        vec![],
    )];
    let output = format_events_output("PROJ-1", None, &events);
    assert!(output.contains("## Event 1 - abc123"));
    assert!(output.contains("**Date:** 2024-01-15T10:00:00Z"));
    assert!(output.contains("**Platform:** python"));
    assert!(output.contains("**Message:** Error occurred"));
    assert!(output.contains("**Tags:** env=prod"));
}

#[test]
fn test_format_events_multiple_tags() {
    let events = vec![make_event(
        "evt1",
        "2024-01-01",
        None,
        None,
        vec![("env", "prod"), ("server", "web-1"), ("release", "1.0.0")],
        vec![],
    )];
    let output = format_events_output("X-1", None, &events);
    assert!(output.contains("env=prod"));
    assert!(output.contains("server=web-1"));
    assert!(output.contains("release=1.0.0"));
}

#[test]
fn test_format_events_with_exception() {
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({
            "values": [
                {"type": "ValueError", "value": "invalid input"}
            ]
        }),
    }];
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-1", None, &events);
    assert!(output.contains("**Exception:** ValueError - invalid input"));
}

#[test]
fn test_format_events_multiple_exceptions() {
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({
            "values": [
                {"type": "KeyError", "value": "'missing_key'"},
                {"type": "RuntimeError", "value": "chain error"}
            ]
        }),
    }];
    let events = vec![make_event("e2", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-2", None, &events);
    assert!(output.contains("**Exception:** KeyError - 'missing_key'"));
    assert!(output.contains("**Exception:** RuntimeError - chain error"));
}

#[test]
fn test_format_events_multiple_events() {
    let events = vec![
        make_event("first", "2024-01-01", None, None, vec![], vec![]),
        make_event("second", "2024-01-02", None, None, vec![], vec![]),
    ];
    let output = format_events_output("P-3", None, &events);
    assert!(output.contains("## Event 1 - first"));
    assert!(output.contains("## Event 2 - second"));
    assert!(output.contains("**Found:** 2 events"));
}

#[test]
fn test_format_events_empty_message_not_shown() {
    let events = vec![make_event(
        "e1",
        "2024-01-01",
        None,
        Some(""),
        vec![],
        vec![],
    )];
    let output = format_events_output("P-1", None, &events);
    assert!(!output.contains("**Message:**"));
}

#[test]
fn test_format_events_no_tags() {
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], vec![])];
    let output = format_events_output("P-1", None, &events);
    assert!(!output.contains("**Tags:**"));
}

#[test]
fn test_format_events_non_exception_entry_ignored() {
    let entries = vec![EventEntry {
        entry_type: "breadcrumbs".to_string(),
        data: json!({"values": []}),
    }];
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-1", None, &events);
    assert!(!output.contains("**Exception:**"));
}

#[test]
fn test_format_events_exception_missing_type() {
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({
            "values": [{"value": "some error"}]
        }),
    }];
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-1", None, &events);
    assert!(output.contains("**Exception:** ? - some error"));
}

#[test]
fn test_format_events_exception_missing_value() {
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({
            "values": [{"type": "CustomError"}]
        }),
    }];
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-1", None, &events);
    assert!(output.contains("**Exception:** CustomError - ?"));
}

#[test]
fn test_format_events_exception_empty_values() {
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({"values": []}),
    }];
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-1", None, &events);
    assert!(!output.contains("**Exception:**"));
}

#[test]
fn test_format_events_exception_no_values_key() {
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({"other": "data"}),
    }];
    let events = vec![make_event("e1", "2024-01-01", None, None, vec![], entries)];
    let output = format_events_output("P-1", None, &events);
    assert!(!output.contains("**Exception:**"));
}

#[test]
fn test_format_events_no_platform() {
    let events = vec![make_event(
        "e1",
        "2024-01-01",
        None,
        Some("msg"),
        vec![],
        vec![],
    )];
    let output = format_events_output("P-1", None, &events);
    assert!(!output.contains("**Platform:**"));
}
