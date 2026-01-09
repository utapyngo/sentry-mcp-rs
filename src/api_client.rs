use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::info;

pub struct SentryApiClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
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
    pub first_seen: String,
    pub last_seen: String,
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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IssueTag {
    pub key: String,
    pub name: String,
    #[serde(rename = "totalValues")]
    pub total_values: i64,
}

#[derive(Debug, Deserialize)]
pub struct EventTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Event {
    pub id: String,
    #[serde(rename = "eventID")]
    pub event_id: String,
    #[serde(rename = "dateCreated")]
    pub date_created: String,
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

#[derive(Debug, Deserialize)]
pub struct EventEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct TraceResponse {
    pub transactions: Vec<TraceTransaction>,
    #[serde(default)]
    pub orphan_errors: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TraceTransaction {
    pub event_id: String,
    pub project_id: i64,
    pub project_slug: String,
    pub transaction: String,
    #[serde(rename = "start_timestamp")]
    pub start_timestamp: f64,
    #[serde(rename = "sdk.name")]
    pub sdk_name: Option<String>,
    pub timestamp: f64,
    #[serde(default)]
    pub children: Vec<TraceTransaction>,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    #[serde(rename = "span.op")]
    pub span_op: Option<String>,
    #[serde(rename = "span.description")]
    pub span_description: Option<String>,
    #[serde(rename = "span.status")]
    pub span_status: Option<String>,
    #[serde(rename = "span.duration")]
    pub span_duration: Option<f64>,
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
            && let Ok(proxy) = reqwest::Proxy::https(&proxy_url) {
                builder = builder.proxy(proxy);
            }
        let client = builder.build().expect("Failed to build HTTP client");
        Self { client, base_url }
    }
    pub async fn get_issue(&self, org_slug: &str, issue_id: &str) -> anyhow::Result<Issue> {
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
            tracing::error!("Failed to parse issue JSON: {}. Response: {}", e, &text[..500.min(text.len())]);
            anyhow::anyhow!("JSON parse error: {}", e)
        })
    }
    pub async fn get_latest_event(&self, org_slug: &str, issue_id: &str) -> anyhow::Result<Event> {
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
            tracing::error!("Failed to parse event JSON: {}. Response: {}", e, &text[..1000.min(text.len())]);
            anyhow::anyhow!("JSON parse error: {}", e)
        })
    }
    pub async fn get_event(
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
    pub async fn get_trace(
        &self,
        org_slug: &str,
        trace_id: &str,
    ) -> anyhow::Result<TraceResponse> {
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
    pub async fn list_events_for_issue(
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
