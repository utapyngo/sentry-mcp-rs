use sentry_mcp::api_client::{
    Event, EventEntry, EventTag, EventsQuery, Issue, IssueTag, Project, TraceResponse,
    TraceTransaction,
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
fn test_trace_response_deserialize_empty() {
    let json = json!({
        "transactions": [],
        "orphan_errors": []
    });
    let trace: TraceResponse = serde_json::from_value(json).unwrap();
    assert!(trace.transactions.is_empty());
    assert!(trace.orphan_errors.is_empty());
}

#[test]
fn test_trace_transaction_deserialize_minimal() {
    let json = json!({
        "eventId": "abc123",
        "projectId": 1,
        "projectSlug": "proj",
        "transaction": "test-tx",
        "start_timestamp": 1000.0,
        "timestamp": 1001.0
    });
    let tx: TraceTransaction = serde_json::from_value(json).unwrap();
    assert_eq!(tx.event_id, "abc123");
    assert_eq!(tx.project_id, 1);
    assert_eq!(tx.project_slug, "proj");
    assert_eq!(tx.transaction, "test-tx");
    assert!(tx.span_op.is_none());
    assert!(tx.children.is_empty());
}

#[test]
fn test_trace_transaction_deserialize_full() {
    let json = json!({
        "eventId": "abc123",
        "projectId": 1,
        "projectSlug": "proj",
        "transaction": "test-tx",
        "start_timestamp": 1000.0,
        "sdk.name": "sentry.python",
        "timestamp": 1001.0,
        "children": [],
        "errors": [{"title": "Error"}],
        "spanId": "span1",
        "parentSpanId": "span0",
        "span.op": "http.request",
        "span.description": "GET /api",
        "span.status": "ok",
        "span.duration": 150.5
    });
    let tx: TraceTransaction = serde_json::from_value(json).unwrap();
    assert_eq!(tx.sdk_name.as_deref(), Some("sentry.python"));
    assert_eq!(tx.span_id.as_deref(), Some("span1"));
    assert_eq!(tx.parent_span_id.as_deref(), Some("span0"));
    assert_eq!(tx.span_op.as_deref(), Some("http.request"));
    assert_eq!(tx.span_description.as_deref(), Some("GET /api"));
    assert_eq!(tx.span_status.as_deref(), Some("ok"));
    assert_eq!(tx.span_duration, Some(150.5));
    assert_eq!(tx.errors.len(), 1);
}

#[test]
fn test_trace_transaction_with_children() {
    let json = json!({
        "eventId": "parent",
        "projectId": 1,
        "projectSlug": "proj",
        "transaction": "parent-tx",
        "start_timestamp": 1000.0,
        "timestamp": 1001.0,
        "children": [{
            "eventId": "child",
            "projectId": 1,
            "projectSlug": "proj",
            "transaction": "child-tx",
            "start_timestamp": 1000.1,
            "timestamp": 1000.5,
            "children": []
        }]
    });
    let tx: TraceTransaction = serde_json::from_value(json).unwrap();
    assert_eq!(tx.children.len(), 1);
    assert_eq!(tx.children[0].event_id, "child");
    assert_eq!(tx.children[0].transaction, "child-tx");
}
