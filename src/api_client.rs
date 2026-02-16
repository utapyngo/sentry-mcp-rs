use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::info;

#[async_trait]
pub trait SentryApi: Send + Sync {
    async fn get_issue(&self, org_slug: &str, issue_id: &str) -> anyhow::Result<Issue>;
    async fn get_latest_event(&self, org_slug: &str, issue_id: &str) -> anyhow::Result<Event>;
    async fn get_event(
        &self,
        org_slug: &str,
        issue_id: &str,
        event_id: &str,
    ) -> anyhow::Result<Event>;
    async fn get_trace(&self, org_slug: &str, trace_id: &str) -> anyhow::Result<TraceResponse>;
    async fn list_events_for_issue(
        &self,
        org_slug: &str,
        issue_id: &str,
        query: &EventsQuery,
    ) -> anyhow::Result<Vec<Event>>;
}

pub struct SentryApiClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Issue {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub culprit: Option<String>,
    pub status: String,
    #[serde(default)]
    pub substatus: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    pub platform: Option<String>,
    pub project: Project,
    #[serde(default)]
    pub first_seen: Option<String>,
    #[serde(default)]
    pub last_seen: Option<String>,
    pub count: String,
    #[serde(rename = "userCount")]
    pub user_count: i64,
    pub permalink: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub tags: Vec<IssueTag>,
    #[serde(default, rename = "issueType")]
    pub issue_type: Option<String>,
    #[serde(default, rename = "issueCategory")]
    pub issue_category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct IssueTag {
    pub key: String,
    pub name: String,
    #[serde(rename = "totalValues")]
    pub total_values: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Event {
    pub id: String,
    #[serde(rename = "eventID")]
    pub event_id: String,
    #[serde(rename = "dateCreated", default)]
    pub date_created: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub entries: Vec<EventEntry>,
    #[serde(default)]
    pub contexts: serde_json::Value,
    #[serde(default)]
    pub context: serde_json::Value,
    #[serde(default)]
    pub tags: Vec<EventTag>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TraceResponse {
    pub transactions: Vec<TraceTransaction>,
    #[serde(default)]
    pub orphan_errors: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TraceTransaction {
    pub event_id: String,
    pub project_id: i64,
    pub project_slug: String,
    pub transaction: String,
    pub start_timestamp: f64,
    pub sdk_name: Option<String>,
    pub timestamp: f64,
    #[serde(default)]
    pub children: Vec<TraceTransaction>,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub parent_event_id: Option<String>,
    #[serde(default)]
    pub generation: i32,
    pub profiler_id: Option<String>,
    #[serde(default)]
    pub performance_issues: Vec<serde_json::Value>,
    #[serde(rename = "transaction.op")]
    pub span_op: Option<String>,
    #[serde(rename = "transaction.duration")]
    pub span_duration: Option<f64>,
    #[serde(default)]
    pub span_description: Option<String>,
    #[serde(default)]
    pub span_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

impl SentryApiClient {
    pub fn new() -> Self {
        let auth_token = env::var("SENTRY_AUTH_TOKEN").expect("SENTRY_AUTH_TOKEN must be set");
        let host = env::var("SENTRY_HOST").unwrap_or_else(|_| "sentry.io".to_string());
        let base_url = format!("https://{}/api/0", host);
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", auth_token)).unwrap(),
        );
        let mut builder = Client::builder().default_headers(headers);
        if let Ok(proxy_url) = env::var("SOCKS_PROXY").or_else(|_| env::var("socks_proxy")) {
            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                builder = builder.proxy(proxy);
            }
        } else if let Ok(proxy_url) = env::var("HTTPS_PROXY").or_else(|_| env::var("https_proxy"))
            && let Ok(proxy) = reqwest::Proxy::https(&proxy_url)
        {
            builder = builder.proxy(proxy);
        }
        let client = builder.build().expect("Failed to build HTTP client");
        Self { client, base_url }
    }
    #[cfg(test)]
    pub fn with_base_url(client: Client, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[async_trait]
impl SentryApi for SentryApiClient {
    async fn get_issue(&self, org_slug: &str, issue_id: &str) -> anyhow::Result<Issue> {
        let url = format!(
            "{}/organizations/{}/issues/{}/",
            self.base_url, org_slug, issue_id
        );
        info!("GET {}", url);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get issue: {} - {}", status, text);
        }
        let text = resp.text().await?;
        serde_json::from_str(&text).map_err(|e| {
            tracing::error!(
                "Failed to parse issue JSON: {}. Response: {}",
                e,
                &text[..500.min(text.len())]
            );
            anyhow::anyhow!("JSON parse error: {}", e)
        })
    }
    async fn get_latest_event(&self, org_slug: &str, issue_id: &str) -> anyhow::Result<Event> {
        let url = format!(
            "{}/organizations/{}/issues/{}/events/latest/",
            self.base_url, org_slug, issue_id
        );
        info!("GET {}", url);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get latest event: {} - {}", status, text);
        }
        let text = resp.text().await?;
        serde_json::from_str(&text).map_err(|e| {
            tracing::error!(
                "Failed to parse event JSON: {}. Response: {}",
                e,
                &text[..1000.min(text.len())]
            );
            anyhow::anyhow!("JSON parse error: {}", e)
        })
    }
    async fn get_event(
        &self,
        org_slug: &str,
        issue_id: &str,
        event_id: &str,
    ) -> anyhow::Result<Event> {
        let url = format!(
            "{}/organizations/{}/issues/{}/events/{}/",
            self.base_url, org_slug, issue_id, event_id
        );
        info!("GET {}", url);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get event: {} - {}", status, text);
        }
        Ok(resp.json().await?)
    }
    async fn get_trace(&self, org_slug: &str, trace_id: &str) -> anyhow::Result<TraceResponse> {
        let url = format!(
            "{}/organizations/{}/events-trace/{}/?limit=100&useSpans=1",
            self.base_url, org_slug, trace_id
        );
        info!("GET {}", url);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get trace: {} - {}", status, text);
        }
        Ok(resp.json().await?)
    }
    async fn list_events_for_issue(
        &self,
        org_slug: &str,
        issue_id: &str,
        query: &EventsQuery,
    ) -> anyhow::Result<Vec<Event>> {
        let mut url = format!(
            "{}/organizations/{}/issues/{}/events/",
            self.base_url, org_slug, issue_id
        );
        let query_string = serde_qs::to_string(query).unwrap_or_default();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }
        info!("GET {}", url);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list events: {} - {}", status, text);
        }
        Ok(resp.json().await?)
    }
}

impl Default for SentryApiClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    #[tokio::test]
    async fn test_get_issue_success() {
        let mock_server = MockServer::start().await;
        let response = r#"{
            "id": "123",
            "shortId": "PROJ-1",
            "title": "Test Error",
            "culprit": "test.py",
            "status": "unresolved",
            "project": {"id": "1", "name": "Test", "slug": "test"},
            "firstSeen": "2024-01-01T00:00:00Z",
            "lastSeen": "2024-01-02T00:00:00Z",
            "count": "42",
            "userCount": 5
        }"#;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/issues/123/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let issue = client.get_issue("test-org", "123").await.unwrap();
        assert_eq!(issue.id, "123");
        assert_eq!(issue.short_id, "PROJ-1");
        assert_eq!(issue.title, "Test Error");
        assert_eq!(issue.count, "42");
    }
    #[tokio::test]
    async fn test_get_issue_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/issues/999/"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let result = client.get_issue("test-org", "999").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }
    #[tokio::test]
    async fn test_get_latest_event_success() {
        let mock_server = MockServer::start().await;
        let response = r#"{
            "id": "ev1",
            "eventID": "abc123",
            "dateCreated": "2024-01-01T00:00:00Z",
            "message": "Test message"
        }"#;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/issues/123/events/latest/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let event = client.get_latest_event("test-org", "123").await.unwrap();
        assert_eq!(event.event_id, "abc123");
        assert_eq!(event.date_created, Some("2024-01-01T00:00:00Z".to_string()));
    }
    #[tokio::test]
    async fn test_get_latest_event_without_date_created() {
        let mock_server = MockServer::start().await;
        let response = r#"{
            "id": "ev1",
            "eventID": "abc123",
            "message": "Test message"
        }"#;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/issues/123/events/latest/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let event = client.get_latest_event("test-org", "123").await.unwrap();
        assert_eq!(event.event_id, "abc123");
        assert!(event.date_created.is_none());
    }
    #[tokio::test]
    async fn test_get_event_success() {
        let mock_server = MockServer::start().await;
        let response = r#"{
            "id": "ev1",
            "eventID": "abc123"
        }"#;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/issues/123/events/abc123/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let event = client.get_event("test-org", "123", "abc123").await.unwrap();
        assert_eq!(event.event_id, "abc123");
    }
    #[tokio::test]
    async fn test_get_trace_success() {
        let mock_server = MockServer::start().await;
        let response = r#"{
            "transactions": [{
                "event_id": "tx1",
                "project_id": 1,
                "project_slug": "test",
                "transaction": "GET /api",
                "start_timestamp": 1704067200.0,
                "timestamp": 1704067201.0
            }],
            "orphan_errors": []
        }"#;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/events-trace/trace123/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let trace = client.get_trace("test-org", "trace123").await.unwrap();
        assert_eq!(trace.transactions.len(), 1);
        assert_eq!(trace.transactions[0].transaction, "GET /api");
    }
    #[tokio::test]
    async fn test_list_events_for_issue_success() {
        let mock_server = MockServer::start().await;
        let response = r#"[
            {"id": "ev1", "eventID": "abc123"},
            {"id": "ev2", "eventID": "def456"}
        ]"#;
        Mock::given(method("GET"))
            .and(path("/organizations/test-org/issues/123/events/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
        let client = SentryApiClient::with_base_url(Client::new(), mock_server.uri());
        let query = EventsQuery {
            query: None,
            limit: Some(10),
            sort: None,
        };
        let events = client
            .list_events_for_issue("test-org", "123", &query)
            .await
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "abc123");
        assert_eq!(events[1].event_id, "def456");
    }
}
