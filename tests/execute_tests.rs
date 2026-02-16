use async_trait::async_trait;
use sentry_mcp::api_client::{
    Event, EventTag, EventsQuery, Issue, IssueTag, Project, SentryApi, TraceMeta, TraceSpan,
};
use sentry_mcp::tools::get_issue_details::{GetIssueDetailsInput, execute as execute_get_issue};
use sentry_mcp::tools::get_trace_details::{GetTraceDetailsInput, execute as execute_get_trace};
use sentry_mcp::tools::search_issue_events::{SearchIssueEventsInput, execute as execute_search};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

struct MockSentryClient {
    issue: Option<Issue>,
    event: Option<Event>,
    trace: Option<Vec<TraceSpan>>,
    trace_meta: Option<TraceMeta>,
    events: Vec<Event>,
    error: Option<String>,
    get_issue_calls: AtomicUsize,
    get_event_calls: AtomicUsize,
    get_latest_event_calls: AtomicUsize,
    get_trace_calls: AtomicUsize,
    get_trace_meta_calls: AtomicUsize,
    list_events_calls: AtomicUsize,
}

impl MockSentryClient {
    fn new() -> Self {
        Self {
            issue: None,
            event: None,
            trace: None,
            trace_meta: None,
            events: vec![],
            error: None,
            get_issue_calls: AtomicUsize::new(0),
            get_event_calls: AtomicUsize::new(0),
            get_latest_event_calls: AtomicUsize::new(0),
            get_trace_calls: AtomicUsize::new(0),
            get_trace_meta_calls: AtomicUsize::new(0),
            list_events_calls: AtomicUsize::new(0),
        }
    }
    fn with_issue(mut self, issue: Issue) -> Self {
        self.issue = Some(issue);
        self
    }
    fn with_event(mut self, event: Event) -> Self {
        self.event = Some(event);
        self
    }
    fn with_trace(mut self, trace: Vec<TraceSpan>) -> Self {
        self.trace = Some(trace);
        self
    }
    fn with_events(mut self, events: Vec<Event>) -> Self {
        self.events = events;
        self
    }
    fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }
}

fn make_issue(id: &str, title: &str) -> Issue {
    Issue {
        id: id.to_string(),
        short_id: format!("PROJ-{}", id),
        title: title.to_string(),
        culprit: Some("test.rs".to_string()),
        permalink: Some(format!("https://sentry.io/issues/{}", id)),
        first_seen: Some("2024-01-01T00:00:00Z".to_string()),
        last_seen: Some("2024-01-02T00:00:00Z".to_string()),
        count: "10".to_string(),
        user_count: 5,
        status: "unresolved".to_string(),
        substatus: None,
        level: Some("error".to_string()),
        platform: Some("rust".to_string()),
        project: Project {
            id: "1".to_string(),
            name: "test-project".to_string(),
            slug: "test-project".to_string(),
        },
        tags: vec![IssueTag {
            key: "environment".to_string(),
            name: "Environment".to_string(),
            total_values: 1,
        }],
        metadata: serde_json::json!({"value": "Test error"}),
        issue_type: Some("error".to_string()),
        issue_category: Some("error".to_string()),
    }
}

fn make_event(id: &str) -> Event {
    Event {
        id: id.to_string(),
        event_id: id.to_string(),
        date_created: Some("2024-01-01T12:00:00Z".to_string()),
        message: Some("Test message".to_string()),
        platform: Some("rust".to_string()),
        tags: vec![EventTag {
            key: "server_name".to_string(),
            value: "web-1".to_string(),
        }],
        entries: vec![],
        contexts: serde_json::json!({}),
        context: serde_json::json!({}),
    }
}

fn make_trace() -> Vec<TraceSpan> {
    vec![TraceSpan {
        event_id: "tx1".to_string(),
        transaction_id: Some("tx1-id".to_string()),
        project_id: 1,
        project_slug: "test-project".to_string(),
        profile_id: None,
        profiler_id: None,
        parent_span_id: None,
        start_timestamp: 1000.0,
        end_timestamp: 1001.0,
        duration: 1000.0,
        transaction: Some("test-transaction".to_string()),
        is_transaction: true,
        description: Some("GET /api/test".to_string()),
        sdk_name: None,
        op: Some("http.server".to_string()),
        name: Some("http.server".to_string()),
        children: vec![],
        errors: vec![],
        occurrences: vec![],
    }]
}

#[async_trait]
impl SentryApi for MockSentryClient {
    async fn get_issue(&self, _org_slug: &str, _issue_id: &str) -> anyhow::Result<Issue> {
        self.get_issue_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = &self.error {
            return Err(anyhow::anyhow!("{}", err));
        }
        self.issue
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Issue not found"))
    }
    async fn get_latest_event(&self, _org_slug: &str, _issue_id: &str) -> anyhow::Result<Event> {
        self.get_latest_event_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = &self.error {
            return Err(anyhow::anyhow!("{}", err));
        }
        self.event
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Event not found"))
    }
    async fn get_event(
        &self,
        _org_slug: &str,
        _issue_id: &str,
        _event_id: &str,
    ) -> anyhow::Result<Event> {
        self.get_event_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = &self.error {
            return Err(anyhow::anyhow!("{}", err));
        }
        self.event
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Event not found"))
    }
    async fn get_trace(&self, _org_slug: &str, _trace_id: &str) -> anyhow::Result<Vec<TraceSpan>> {
        self.get_trace_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = &self.error {
            return Err(anyhow::anyhow!("{}", err));
        }
        self.trace
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Trace not found"))
    }
    async fn get_trace_meta(&self, _org_slug: &str, _trace_id: &str) -> anyhow::Result<TraceMeta> {
        self.get_trace_meta_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = &self.error {
            return Err(anyhow::anyhow!("{}", err));
        }
        Ok(self.trace_meta.clone().unwrap_or(TraceMeta {
            logs: 0,
            errors: 0,
            performance_issues: 0,
            span_count: 0.0,
            span_count_map: HashMap::new(),
        }))
    }
    async fn list_events_for_issue(
        &self,
        _org_slug: &str,
        _issue_id: &str,
        _query: &EventsQuery,
    ) -> anyhow::Result<Vec<Event>> {
        self.list_events_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = &self.error {
            return Err(anyhow::anyhow!("{}", err));
        }
        Ok(self.events.clone())
    }
}

#[tokio::test]
async fn test_execute_get_issue_basic() {
    let client = MockSentryClient::new()
        .with_issue(make_issue("123", "Test Error"))
        .with_event(make_event("evt1"));
    let input = GetIssueDetailsInput {
        issue_url: None,
        organization_slug: Some("test-org".to_string()),
        issue_id: Some("123".to_string()),
        event_id: None,
    };
    let result = execute_get_issue(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(client.get_issue_calls.load(Ordering::SeqCst), 1);
    assert_eq!(client.get_latest_event_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_get_issue_with_specific_event() {
    let client = MockSentryClient::new()
        .with_issue(make_issue("123", "Test Error"))
        .with_event(make_event("evt1"));
    let input = GetIssueDetailsInput {
        issue_url: None,
        organization_slug: Some("test-org".to_string()),
        issue_id: Some("123".to_string()),
        event_id: Some("evt1".to_string()),
    };
    let result = execute_get_issue(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(client.get_issue_calls.load(Ordering::SeqCst), 1);
    assert_eq!(client.get_event_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_get_issue_from_url() {
    let client = MockSentryClient::new()
        .with_issue(make_issue("123", "Test Error"))
        .with_event(make_event("evt1"));
    let input = GetIssueDetailsInput {
        issue_url: Some("https://sentry.io/organizations/test-org/issues/123/".to_string()),
        organization_slug: None,
        issue_id: None,
        event_id: None,
    };
    let result = execute_get_issue(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(client.get_issue_calls.load(Ordering::SeqCst), 1);
    assert_eq!(client.get_latest_event_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_get_issue_url_with_event_id() {
    let client = MockSentryClient::new()
        .with_issue(make_issue("123", "Test Error"))
        .with_event(make_event("abc123def456"));
    let input = GetIssueDetailsInput {
        issue_url: Some("https://sentry.io/organizations/test-org/issues/123/".to_string()),
        organization_slug: None,
        issue_id: None,
        event_id: Some("abc123def456".to_string()),
    };
    let result = execute_get_issue(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(client.get_issue_calls.load(Ordering::SeqCst), 1);
    assert_eq!(client.get_event_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_get_issue_missing_params() {
    let client = MockSentryClient::new();
    let input = GetIssueDetailsInput {
        issue_url: None,
        organization_slug: None,
        issue_id: None,
        event_id: None,
    };
    let result = execute_get_issue(&client, input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_get_issue_api_error() {
    let client = MockSentryClient::new().with_error("API rate limit exceeded");
    let input = GetIssueDetailsInput {
        issue_url: None,
        organization_slug: Some("test-org".to_string()),
        issue_id: Some("123".to_string()),
        event_id: None,
    };
    let result = execute_get_issue(&client, input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_get_trace_basic() {
    let client = MockSentryClient::new().with_trace(make_trace());
    let input = GetTraceDetailsInput {
        organization_slug: "test-org".to_string(),
        trace_id: "abc123".to_string(),
    };
    let result = execute_get_trace(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(client.get_trace_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_get_trace_api_error() {
    let client = MockSentryClient::new().with_error("Trace not found");
    let input = GetTraceDetailsInput {
        organization_slug: "test-org".to_string(),
        trace_id: "abc123".to_string(),
    };
    let result = execute_get_trace(&client, input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_search_events_basic() {
    let client = MockSentryClient::new().with_events(vec![make_event("evt1"), make_event("evt2")]);
    let input = SearchIssueEventsInput {
        organization_slug: "test-org".to_string(),
        issue_id: "123".to_string(),
        query: None,
        limit: None,
        sort: None,
    };
    let result = execute_search(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(client.list_events_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_search_events_with_query() {
    let client = MockSentryClient::new().with_events(vec![make_event("evt1")]);
    let input = SearchIssueEventsInput {
        organization_slug: "test-org".to_string(),
        issue_id: "123".to_string(),
        query: Some("environment:production".to_string()),
        limit: Some(5),
        sort: Some("oldest".to_string()),
    };
    let result = execute_search(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_execute_search_events_empty() {
    let client = MockSentryClient::new().with_events(vec![]);
    let input = SearchIssueEventsInput {
        organization_slug: "test-org".to_string(),
        issue_id: "123".to_string(),
        query: Some("nonexistent:value".to_string()),
        limit: None,
        sort: None,
    };
    let result = execute_search(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_execute_search_events_api_error() {
    let client = MockSentryClient::new().with_error("Issue not found");
    let input = SearchIssueEventsInput {
        organization_slug: "test-org".to_string(),
        issue_id: "999".to_string(),
        query: None,
        limit: None,
        sort: None,
    };
    let result = execute_search(&client, input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_get_issue_output_contains_issue_details() {
    let client = MockSentryClient::new()
        .with_issue(make_issue("123", "Test Error Title"))
        .with_event(make_event("evt1"));
    let input = GetIssueDetailsInput {
        issue_url: None,
        organization_slug: Some("test-org".to_string()),
        issue_id: Some("123".to_string()),
        event_id: None,
    };
    let result = execute_get_issue(&client, input).await.unwrap();
    let content = &result.content[0];
    if let rmcp::model::RawContent::Text(text) = &content.raw {
        assert!(text.text.contains("Test Error Title"));
        assert!(text.text.contains("PROJ-123"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_execute_get_trace_output_contains_trace_details() {
    let client = MockSentryClient::new().with_trace(make_trace());
    let input = GetTraceDetailsInput {
        organization_slug: "test-org".to_string(),
        trace_id: "abc123".to_string(),
    };
    let result = execute_get_trace(&client, input).await.unwrap();
    let content = &result.content[0];
    if let rmcp::model::RawContent::Text(text) = &content.raw {
        assert!(text.text.contains("abc123"));
        assert!(text.text.contains("GET /api/test"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_execute_search_output_contains_events() {
    let client = MockSentryClient::new().with_events(vec![make_event("evt1")]);
    let input = SearchIssueEventsInput {
        organization_slug: "test-org".to_string(),
        issue_id: "123".to_string(),
        query: None,
        limit: None,
        sort: None,
    };
    let result = execute_search(&client, input).await.unwrap();
    let content = &result.content[0];
    if let rmcp::model::RawContent::Text(text) = &content.raw {
        assert!(text.text.contains("evt1"));
        assert!(text.text.contains("123"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_execute_get_issue_invalid_url() {
    let client = MockSentryClient::new();
    let input = GetIssueDetailsInput {
        issue_url: Some("https://invalid-url.com/not-sentry".to_string()),
        organization_slug: None,
        issue_id: None,
        event_id: None,
    };
    let result = execute_get_issue(&client, input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_search_limit_capped() {
    let client = MockSentryClient::new().with_events(vec![]);
    let input = SearchIssueEventsInput {
        organization_slug: "test-org".to_string(),
        issue_id: "123".to_string(),
        query: None,
        limit: Some(1000),
        sort: None,
    };
    let result = execute_search(&client, input).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
}
