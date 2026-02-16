use sentry_mcp::api_client::{
    Event, EventEntry, EventTag, EventsQuery, Issue, IssueTag, Project, TraceMeta, TraceSpan,
};
use serde_json::json;

#[test]
fn test_issue_deserialize_minimal() {
    let json = json!({
        "id": "12345",
        "shortId": "PROJ-1",
        "title": "Test Issue",
        "status": "unresolved",
        "project": {"id": "1", "name": "Project", "slug": "proj"},
        "firstSeen": "2024-01-01T00:00:00Z",
        "lastSeen": "2024-01-02T00:00:00Z",
        "count": "42",
        "userCount": 5
    });
    let issue: Issue = serde_json::from_value(json).unwrap();
    assert_eq!(issue.id, "12345");
    assert_eq!(issue.short_id, "PROJ-1");
    assert_eq!(issue.title, "Test Issue");
    assert_eq!(issue.status, "unresolved");
    assert!(issue.culprit.is_none());
    assert!(issue.substatus.is_none());
}

#[test]
fn test_issue_deserialize_full() {
    let json = json!({
        "id": "12345",
        "shortId": "PROJ-1",
        "title": "Test Issue",
        "culprit": "app.main",
        "status": "resolved",
        "substatus": "ongoing",
        "level": "error",
        "platform": "python",
        "project": {"id": "1", "name": "Project", "slug": "proj"},
        "firstSeen": "2024-01-01T00:00:00Z",
        "lastSeen": "2024-01-02T00:00:00Z",
        "count": "100",
        "userCount": 10,
        "permalink": "https://sentry.io/issues/12345",
        "metadata": {"value": "KeyError"},
        "tags": [{"key": "env", "name": "Environment", "totalValues": 2}],
        "issueType": "error",
        "issueCategory": "error"
    });
    let issue: Issue = serde_json::from_value(json).unwrap();
    assert_eq!(issue.culprit.as_deref(), Some("app.main"));
    assert_eq!(issue.substatus.as_deref(), Some("ongoing"));
    assert_eq!(issue.level.as_deref(), Some("error"));
    assert_eq!(issue.platform.as_deref(), Some("python"));
    assert_eq!(issue.tags.len(), 1);
    assert_eq!(issue.tags[0].key, "env");
}

#[test]
fn test_project_deserialize() {
    let json = json!({
        "id": "123",
        "name": "My Project",
        "slug": "my-project"
    });
    let project: Project = serde_json::from_value(json).unwrap();
    assert_eq!(project.id, "123");
    assert_eq!(project.name, "My Project");
    assert_eq!(project.slug, "my-project");
}

#[test]
fn test_issue_tag_deserialize() {
    let json = json!({
        "key": "environment",
        "name": "Environment",
        "totalValues": 3
    });
    let tag: IssueTag = serde_json::from_value(json).unwrap();
    assert_eq!(tag.key, "environment");
    assert_eq!(tag.name, "Environment");
    assert_eq!(tag.total_values, 3);
}

#[test]
fn test_event_tag_deserialize() {
    let json = json!({
        "key": "browser",
        "value": "Chrome 120"
    });
    let tag: EventTag = serde_json::from_value(json).unwrap();
    assert_eq!(tag.key, "browser");
    assert_eq!(tag.value, "Chrome 120");
}

#[test]
fn test_event_deserialize_minimal() {
    let json = json!({
        "id": "abc123",
        "eventID": "abc123",
        "dateCreated": "2024-01-01T00:00:00Z"
    });
    let event: Event = serde_json::from_value(json).unwrap();
    assert_eq!(event.id, "abc123");
    assert_eq!(event.event_id, "abc123");
    assert_eq!(event.date_created, Some("2024-01-01T00:00:00Z".to_string()));
    assert!(event.message.is_none());
    assert!(event.entries.is_empty());
}

#[test]
fn test_event_deserialize_full() {
    let json = json!({
        "id": "abc123",
        "eventID": "abc123",
        "dateCreated": "2024-01-01T00:00:00Z",
        "message": "Test message",
        "platform": "python",
        "entries": [{"type": "exception", "data": {"values": []}}],
        "contexts": {"browser": {"name": "Chrome"}},
        "context": {"extra": "data"},
        "tags": [{"key": "env", "value": "prod"}]
    });
    let event: Event = serde_json::from_value(json).unwrap();
    assert_eq!(event.message.as_deref(), Some("Test message"));
    assert_eq!(event.platform.as_deref(), Some("python"));
    assert_eq!(event.entries.len(), 1);
    assert_eq!(event.tags.len(), 1);
}

#[test]
fn test_event_entry_deserialize() {
    let json = json!({
        "type": "exception",
        "data": {"values": [{"type": "RuntimeError", "value": "test"}]}
    });
    let entry: EventEntry = serde_json::from_value(json).unwrap();
    assert_eq!(entry.entry_type, "exception");
    assert!(entry.data.get("values").is_some());
}

#[test]
fn test_events_query_serialize_empty() {
    let query = EventsQuery {
        query: None,
        limit: None,
        sort: None,
    };
    let serialized = serde_json::to_value(&query).unwrap();
    assert_eq!(serialized, json!({}));
}

#[test]
fn test_events_query_serialize_full() {
    let query = EventsQuery {
        query: Some("browser:Chrome".to_string()),
        limit: Some(50),
        sort: Some("oldest".to_string()),
    };
    let serialized = serde_json::to_value(&query).unwrap();
    assert_eq!(serialized["query"], "browser:Chrome");
    assert_eq!(serialized["limit"], 50);
    assert_eq!(serialized["sort"], "oldest");
}

#[test]
fn test_trace_span_deserialize_minimal() {
    let json = json!({
        "event_id": "abc123",
        "project_id": 1,
        "project_slug": "proj",
        "parent_span_id": null,
        "start_timestamp": 1000.0,
        "duration": 100.0
    });
    let span: TraceSpan = serde_json::from_value(json).unwrap();
    assert_eq!(span.event_id, "abc123");
    assert_eq!(span.project_id, 1);
    assert_eq!(span.project_slug, "proj");
    assert!(!span.is_transaction);
    assert!(span.children.is_empty());
    assert!(span.op.is_none());
}

#[test]
fn test_trace_span_deserialize_full() {
    let json = json!({
        "event_id": "91958dc2ae005f54",
        "transaction_id": "4ff9a0a8138a447c9e0572a2eeff55d8",
        "project_id": 19,
        "project_slug": "platform_test_project",
        "profile_id": "",
        "profiler_id": "",
        "parent_span_id": "91958dc2ae005f54",
        "start_timestamp": 1771164551.506854,
        "end_timestamp": 1771164551.506973,
        "duration": 326.0,
        "transaction": "/api/resource/{id}/",
        "is_transaction": true,
        "description": "/api/resource/{id}/",
        "sdk_name": "sentry.python.django",
        "op": "http.server",
        "name": "http.server",
        "children": [],
        "errors": [],
        "occurrences": []
    });
    let span: TraceSpan = serde_json::from_value(json).unwrap();
    assert_eq!(span.event_id, "91958dc2ae005f54");
    assert_eq!(
        span.transaction_id.as_deref(),
        Some("4ff9a0a8138a447c9e0572a2eeff55d8")
    );
    assert_eq!(span.project_id, 19);
    assert!(span.is_transaction);
    assert_eq!(span.op.as_deref(), Some("http.server"));
    assert_eq!(span.duration, 326.0);
    assert_eq!(span.sdk_name.as_deref(), Some("sentry.python.django"));
}

#[test]
fn test_trace_span_with_children() {
    let json = json!({
        "event_id": "parent",
        "project_id": 1,
        "project_slug": "proj",
        "parent_span_id": null,
        "start_timestamp": 1000.0,
        "end_timestamp": 1001.0,
        "duration": 1000.0,
        "is_transaction": true,
        "op": "http.server",
        "children": [{
            "event_id": "child",
            "project_id": 1,
            "project_slug": "proj",
            "parent_span_id": "parent",
            "start_timestamp": 1000.1,
            "end_timestamp": 1000.5,
            "duration": 400.0,
            "op": "db",
            "children": []
        }]
    });
    let span: TraceSpan = serde_json::from_value(json).unwrap();
    assert_eq!(span.children.len(), 1);
    assert_eq!(span.children[0].event_id, "child");
    assert_eq!(span.children[0].op.as_deref(), Some("db"));
}

#[test]
fn test_trace_response_is_vec() {
    let json = json!([
        {
            "event_id": "span1",
            "project_id": 1,
            "project_slug": "proj",
            "parent_span_id": null,
            "start_timestamp": 1000.0,
            "duration": 100.0,
            "is_transaction": true,
            "children": []
        },
        {
            "event_id": "span2",
            "project_id": 1,
            "project_slug": "proj",
            "parent_span_id": null,
            "start_timestamp": 1001.0,
            "duration": 200.0,
            "children": []
        }
    ]);
    let spans: Vec<TraceSpan> = serde_json::from_value(json).unwrap();
    assert_eq!(spans.len(), 2);
    assert!(spans[0].is_transaction);
    assert!(!spans[1].is_transaction);
}

#[test]
fn test_trace_meta_deserialize() {
    let json = json!({
        "logs": 0,
        "errors": 2,
        "performance_issues": 1,
        "span_count": 1122.0,
        "span_count_map": {
            "event.django": 730.0,
            "db": 184.0
        }
    });
    let meta: TraceMeta = serde_json::from_value(json).unwrap();
    assert_eq!(meta.logs, 0);
    assert_eq!(meta.errors, 2);
    assert_eq!(meta.performance_issues, 1);
    assert_eq!(meta.span_count, 1122.0);
    assert_eq!(meta.span_count_map.get("db"), Some(&184.0));
}

#[test]
fn test_trace_meta_deserialize_minimal() {
    let json = json!({});
    let meta: TraceMeta = serde_json::from_value(json).unwrap();
    assert_eq!(meta.errors, 0);
    assert_eq!(meta.span_count, 0.0);
    assert!(meta.span_count_map.is_empty());
}

