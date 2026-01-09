use sentry_rs::api_client::EventEntry;
use sentry_rs::tools::get_issue_details::{
    format_contexts, format_event_entries, format_exception, format_extra_data,
    format_frame_detail, parse_issue_url,
};
use serde_json::json;

#[test]
fn test_parse_issue_url_valid() {
    let url = "https://sentry.io/organizations/myorg/issues/12345/";
    let result = parse_issue_url(url);
    assert!(result.is_some());
    let (org, issue) = result.unwrap();
    assert_eq!(org, "myorg");
    assert_eq!(issue, "12345");
}

#[test]
fn test_parse_issue_url_with_query() {
    let url = "https://sentry.io/organizations/testorg/issues/99999?referrer=issue-stream";
    let result = parse_issue_url(url);
    assert!(result.is_some());
    let (org, issue) = result.unwrap();
    assert_eq!(org, "testorg");
    assert_eq!(issue, "99999");
}

#[test]
fn test_parse_issue_url_custom_domain() {
    let url = "https://sentry.example.com/organizations/corp/issues/42/";
    let result = parse_issue_url(url);
    assert!(result.is_some());
    let (org, issue) = result.unwrap();
    assert_eq!(org, "corp");
    assert_eq!(issue, "42");
}

#[test]
fn test_parse_issue_url_invalid() {
    let url = "https://sentry.io/not/valid/url";
    let result = parse_issue_url(url);
    assert!(result.is_none());
}

#[test]
fn test_parse_issue_url_not_url() {
    let result = parse_issue_url("not-a-url");
    assert!(result.is_none());
}

#[test]
fn test_format_extra_data_simple() {
    let mut output = String::new();
    let extra: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({"key1": "value1", "key2": 123})).unwrap();
    format_extra_data(&mut output, &extra);
    assert!(output.contains("### Extra Data"));
    assert!(output.contains("**key1:**"));
    assert!(output.contains("**key2:**"));
}

#[test]
fn test_format_extra_data_with_array() {
    let mut output = String::new();
    let extra: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({"items": ["a", "b", "c"]})).unwrap();
    format_extra_data(&mut output, &extra);
    assert!(output.contains("### Extra Data"));
    assert!(output.contains("**items:**"));
}

#[test]
fn test_format_contexts_simple() {
    let mut output = String::new();
    let contexts: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({
            "browser": {"name": "Chrome", "version": "120.0"},
            "os": {"name": "Linux"}
        }))
        .unwrap();
    format_contexts(&mut output, &contexts);
    assert!(output.contains("### Context"));
    assert!(output.contains("**browser:**"));
    assert!(output.contains("Chrome"));
}

#[test]
fn test_format_contexts_nested() {
    let mut output = String::new();
    let contexts: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({
            "runtime": {"name": "python", "version": "3.11.0"}
        }))
        .unwrap();
    format_contexts(&mut output, &contexts);
    assert!(output.contains("**runtime:**"));
    assert!(output.contains("python"));
    assert!(output.contains("3.11.0"));
}

#[test]
fn test_format_frame_detail_simple() {
    let mut output = String::new();
    let frame = json!({
        "filename": "app.py",
        "lineNo": 42,
        "function": "main"
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("app.py"));
    assert!(output.contains("42"));
    assert!(output.contains("main"));
}

#[test]
fn test_format_frame_detail_with_context() {
    let mut output = String::new();
    let frame = json!({
        "filename": "app.py",
        "lineNo": 42,
        "function": "main",
        "context": [
            [41, "def main():"],
            [42, "    raise ValueError()"],
            [43, "    return"]
        ]
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("→")); // current line marker
    assert!(output.contains("raise ValueError"));
}

#[test]
fn test_format_frame_detail_with_vars() {
    let mut output = String::new();
    let frame = json!({
        "filename": "app.py",
        "lineNo": 10,
        "function": "test",
        "vars": {
            "x": 123,
            "y": "hello"
        }
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("Local Variables"));
    assert!(output.contains("x:"));
    assert!(output.contains("y:"));
}

#[test]
fn test_format_frame_detail_truncates_long_vars() {
    let mut output = String::new();
    let long_value = "a".repeat(100);
    let frame = json!({
        "filename": "app.py",
        "lineNo": 10,
        "function": "test",
        "vars": {
            "long_var": long_value
        }
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("..."));
}

#[test]
fn test_format_exception_simple() {
    let mut output = String::new();
    let exc = json!({
        "type": "ValueError",
        "value": "invalid argument"
    });
    format_exception(&mut output, &exc);
    assert!(output.contains("ValueError"));
    assert!(output.contains("invalid argument"));
}

#[test]
fn test_format_exception_with_stacktrace() {
    let mut output = String::new();
    let exc = json!({
        "type": "KeyError",
        "value": "'missing_key'",
        "stacktrace": {
            "frames": [
                {
                    "filename": "lib.py",
                    "lineNo": 5,
                    "function": "helper",
                    "inApp": false
                },
                {
                    "filename": "main.py",
                    "lineNo": 20,
                    "function": "process",
                    "inApp": true,
                    "context": [[20, "data['missing_key']"]]
                }
            ]
        }
    });
    format_exception(&mut output, &exc);
    assert!(output.contains("KeyError"));
    assert!(output.contains("Most Relevant Frame"));
    assert!(output.contains("main.py"));
    assert!(output.contains("Full Stacktrace"));
}

#[test]
fn test_format_event_entries_exception() {
    let mut output = String::new();
    let entries = vec![EventEntry {
        entry_type: "exception".to_string(),
        data: json!({
            "values": [
                {"type": "RuntimeError", "value": "test error"}
            ]
        }),
    }];
    format_event_entries(&mut output, &entries);
    assert!(output.contains("RuntimeError"));
    assert!(output.contains("test error"));
}

#[test]
fn test_format_event_entries_message() {
    let mut output = String::new();
    let entries = vec![EventEntry {
        entry_type: "message".to_string(),
        data: json!({
            "formatted": "User logged in from unknown location"
        }),
    }];
    format_event_entries(&mut output, &entries);
    assert!(output.contains("Message"));
    assert!(output.contains("User logged in"));
}

#[test]
fn test_format_event_entries_empty() {
    let mut output = String::new();
    let entries: Vec<EventEntry> = vec![];
    format_event_entries(&mut output, &entries);
    assert!(output.is_empty());
}

#[test]
fn test_format_event_entries_unknown_type() {
    let mut output = String::new();
    let entries = vec![EventEntry {
        entry_type: "breadcrumbs".to_string(),
        data: json!({"values": []}),
    }];
    format_event_entries(&mut output, &entries);
    assert!(output.is_empty());
}

#[test]
fn test_format_extra_data_with_null() {
    let mut output = String::new();
    let extra: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({"null_value": null})).unwrap();
    format_extra_data(&mut output, &extra);
    assert!(output.contains("null_value"));
}

#[test]
fn test_format_contexts_non_object() {
    let mut output = String::new();
    let contexts: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({"simple": "string_value"})).unwrap();
    format_contexts(&mut output, &contexts);
    assert!(output.contains("### Context"));
}
