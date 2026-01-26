use sentry_mcp::api_client::{Event, EventEntry, EventTag, Issue, IssueTag, Project};
use sentry_mcp::tools::get_issue_details::{
    format_contexts, format_event_entries, format_exception, format_extra_data,
    format_frame_detail, format_issue_output, parse_issue_url,
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

fn create_test_project() -> Project {
    Project {
        id: "1".to_string(),
        name: "test-project".to_string(),
        slug: "test-project".to_string(),
    }
}

fn create_test_issue(project: Project) -> Issue {
    Issue {
        id: "123".to_string(),
        short_id: "TEST-1".to_string(),
        title: "Test Issue".to_string(),
        culprit: Some("app.main".to_string()),
        status: "unresolved".to_string(),
        substatus: Some("ongoing".to_string()),
        level: Some("error".to_string()),
        platform: Some("python".to_string()),
        project,
        first_seen: "2024-01-01T00:00:00Z".to_string(),
        last_seen: "2024-01-02T00:00:00Z".to_string(),
        count: "42".to_string(),
        user_count: 10,
        permalink: Some("https://sentry.io/issues/123".to_string()),
        metadata: json!({"value": "KeyError: 'foo'"}),
        tags: vec![
            IssueTag {
                key: "environment".to_string(),
                name: "Environment".to_string(),
                total_values: 2,
            },
        ],
        issue_type: Some("error".to_string()),
        issue_category: Some("error".to_string()),
    }
}

fn create_test_event() -> Event {
    Event {
        id: "abc123".to_string(),
        event_id: "abc123".to_string(),
        date_created: Some("2024-01-02T00:00:00Z".to_string()),
        message: Some("Test message".to_string()),
        platform: Some("python".to_string()),
        entries: vec![],
        contexts: json!({}),
        context: json!({}),
        tags: vec![
            EventTag {
                key: "browser".to_string(),
                value: "Chrome".to_string(),
            },
        ],
    }
}

#[test]
fn test_format_issue_output_basic() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("# Issue Details"));
    assert!(output.contains("**ID:** TEST-1"));
    assert!(output.contains("**Title:** Test Issue"));
    assert!(output.contains("**Status:** unresolved"));
    assert!(output.contains("**Level:** error"));
    assert!(output.contains("**Platform:** python"));
    assert!(output.contains("**First Seen:**"));
    assert!(output.contains("**Last Seen:**"));
    assert!(output.contains("**Event Count:** 42"));
    assert!(output.contains("**User Count:** 10"));
}

#[test]
fn test_format_issue_output_with_culprit() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("**Culprit:** app.main"));
}

#[test]
fn test_format_issue_output_with_permalink() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("**URL:**"));
    assert!(output.contains("https://sentry.io/issues/123"));
}

#[test]
fn test_format_issue_output_with_issue_tags() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("## Tags"));
    assert!(output.contains("environment"));
    assert!(output.contains("Environment"));
}

#[test]
fn test_format_issue_output_with_event_tags() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("### Event Tags"));
    assert!(output.contains("browser"));
    assert!(output.contains("Chrome"));
}

#[test]
fn test_format_issue_output_no_culprit() {
    let project = create_test_project();
    let mut issue = create_test_issue(project);
    issue.culprit = None;
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(!output.contains("**Culprit:**"));
}

#[test]
fn test_format_issue_output_no_substatus() {
    let project = create_test_project();
    let mut issue = create_test_issue(project);
    issue.substatus = None;
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("**Status:** unresolved"));
    assert!(!output.contains("**Substatus:**"));
}

#[test]
fn test_format_issue_output_no_permalink() {
    let project = create_test_project();
    let mut issue = create_test_issue(project);
    issue.permalink = None;
    let event = create_test_event();
    let output = format_issue_output(&issue, &event);
    assert!(!output.contains("**URL:**"));
}

#[test]
fn test_format_issue_output_empty_tags() {
    let project = create_test_project();
    let mut issue = create_test_issue(project);
    issue.tags = vec![];
    let mut event = create_test_event();
    event.tags = vec![];
    let output = format_issue_output(&issue, &event);
    assert!(!output.contains("## Tags"));
    assert!(!output.contains("### Event Tags"));
}

#[test]
fn test_format_issue_output_with_event_entries() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let mut event = create_test_event();
    event.entries = vec![EventEntry {
        entry_type: "message".to_string(),
        data: json!({"formatted": "Test message content"}),
    }];
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("## Message"));
    assert!(output.contains("Test message content"));
}

#[test]
fn test_format_frame_detail_with_long_variable() {
    let mut output = String::new();
    let frame = json!({
        "filename": "test.py",
        "lineNo": 10,
        "function": "test_func",
        "vars": {
            "very_long_value": "This is a very long string that should be truncated to fit within the display limit for better readability in the output"
        }
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("very_long_value"));
    assert!(output.contains("..."));
}

#[test]
fn test_format_frame_detail_with_null_variable() {
    let mut output = String::new();
    let frame = json!({
        "filename": "test.py",
        "lineNo": 10,
        "function": "test_func",
        "vars": {"null_var": null}
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("null_var"));
    assert!(output.contains("None"));
}

#[test]
fn test_format_frame_detail_empty_vars() {
    let mut output = String::new();
    let frame = json!({
        "filename": "test.py",
        "lineNo": 10,
        "function": "test_func",
        "vars": {}
    });
    format_frame_detail(&mut output, &frame);
    assert!(output.contains("test.py"));
    assert!(!output.contains("Local Variables"));
}

#[test]
fn test_format_exception_no_stacktrace() {
    let mut output = String::new();
    let exc = json!({
        "type": "ValueError",
        "value": "invalid literal"
    });
    format_exception(&mut output, &exc);
    assert!(output.contains("ValueError"));
    assert!(output.contains("invalid literal"));
    assert!(!output.contains("Stacktrace"));
}

#[test]
fn test_format_exception_empty_frames() {
    let mut output = String::new();
    let exc = json!({
        "type": "Exception",
        "value": "error",
        "stacktrace": {"frames": []}
    });
    format_exception(&mut output, &exc);
    assert!(output.contains("Exception"));
    assert!(!output.contains("Most Relevant Frame"));
}

#[test]
fn test_format_exception_no_in_app_frames() {
    let mut output = String::new();
    let exc = json!({
        "type": "RuntimeError",
        "value": "test",
        "stacktrace": {
            "frames": [{
                "filename": "library.py",
                "lineNo": 100,
                "function": "lib_func",
                "inApp": false
            }]
        }
    });
    format_exception(&mut output, &exc);
    assert!(output.contains("RuntimeError"));
    assert!(output.contains("Full Stacktrace"));
    assert!(!output.contains("Most Relevant Frame"));
}

#[test]
fn test_format_extra_data_with_nested_object() {
    let mut output = String::new();
    let extra: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(json!({"nested": {"a": 1, "b": 2}})).unwrap();
    format_extra_data(&mut output, &extra);
    assert!(output.contains("**nested:**"));
}

#[test]
fn test_format_issue_output_with_contexts() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let mut event = create_test_event();
    event.contexts = json!({
        "browser": {"name": "Firefox", "version": "120"}
    });
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("### Context"));
    assert!(output.contains("Firefox"));
}

#[test]
fn test_format_issue_output_with_extra_context() {
    let project = create_test_project();
    let issue = create_test_issue(project);
    let mut event = create_test_event();
    event.context = json!({"custom_key": "custom_value"});
    let output = format_issue_output(&issue, &event);
    assert!(output.contains("### Extra Data"));
    assert!(output.contains("custom_key"));
}
