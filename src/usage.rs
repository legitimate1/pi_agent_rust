//! Provider account usage and quota readers used by `pi usage` and `/usage`.
//!
//! This module performs bounded, read-only provider requests and normalizes the
//! responses into a small status surface. It deliberately does not participate
//! in provider streaming or persist quota data.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::auth::AuthStorage;
use crate::error::{Error, Result};
use crate::http::client::Client;

/// Schema identifier for machine-readable usage output.
pub const USAGE_SCHEMA: &str = "pi.usage.v1";

/// Duration for which a successful provider reading is considered fresh.
pub const USAGE_CACHE_TTL: Duration = Duration::from_secs(60);

/// Maximum time allowed for an individual provider reading.
pub const USAGE_FETCH_TIMEOUT: Duration = Duration::from_secs(8);

/// Normalized account usage information for one provider.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    /// Canonical provider identifier.
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Endpoint used for the successful read. No credential is included.
    pub source: String,
    pub fetched_at_ms: i64,
    /// Present when the row was served from memory rather than fetched live.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_age_secs: Option<u64>,
}

/// Result for one configured provider.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum UsageStatus {
    /// A live or cached provider reading.
    Ready(ProviderUsage),
    /// The provider is authenticated but has no supported public endpoint.
    Unavailable { provider: String, reason: String },
    /// The provider endpoint could not be read and no cached row was available.
    Error { provider: String, error: String },
}

impl UsageStatus {
    /// Return the canonical provider identifier represented by this row.
    #[must_use]
    pub fn provider(&self) -> &str {
        match self {
            Self::Ready(usage) => &usage.provider,
            Self::Unavailable { provider, .. } | Self::Error { provider, .. } => provider,
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

/// A single provider quota endpoint reader.
#[async_trait::async_trait]
pub trait UsageReader: Send + Sync {
    /// Return the canonical provider identifier.
    fn provider(&self) -> &'static str;

    /// Fetch and normalize one provider reading.
    async fn fetch(&self, client: &Client) -> Result<ProviderUsage>;
}

/// Reads OpenRouter purchased and consumed credits from `/api/v1/credits`.
pub struct OpenRouterUsageReader {
    api_key: String,
    base_url: String,
}

impl OpenRouterUsageReader {
    /// Construct a reader using the production OpenRouter endpoint.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://openrouter.ai".to_string())
    }

    /// Construct a reader with an explicit base URL, primarily for local tests.
    #[must_use]
    pub const fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

#[async_trait::async_trait]
impl UsageReader for OpenRouterUsageReader {
    fn provider(&self) -> &'static str {
        "openrouter"
    }

    async fn fetch(&self, client: &Client) -> Result<ProviderUsage> {
        let url = format!("{}/api/v1/credits", self.base_url.trim_end_matches('/'));
        let body = get_json(
            client,
            &url,
            &[("Authorization", format!("Bearer {}", self.api_key))],
        )
        .await?;
        let data = &body["data"];
        let total = data["total_credits"].as_f64();
        let used = data["total_usage"].as_f64();
        Ok(ProviderUsage {
            provider: "openrouter".to_string(),
            plan: None,
            used,
            limit: total,
            remaining: total.zip(used).map(|(total, used)| total - used),
            unit: Some("USD credits".to_string()),
            resets_at: None,
            detail: None,
            source: url,
            fetched_at_ms: now_ms(),
            cache_age_secs: None,
        })
    }
}

/// Reads Moonshot/Kimi available balance from `/v1/users/me/balance`.
pub struct MoonshotUsageReader {
    api_key: String,
    base_url: String,
}

impl MoonshotUsageReader {
    /// Construct a reader using the production Moonshot endpoint.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://api.moonshot.ai".to_string())
    }

    /// Construct a reader with an explicit base URL, primarily for local tests.
    #[must_use]
    pub const fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

#[async_trait::async_trait]
impl UsageReader for MoonshotUsageReader {
    fn provider(&self) -> &'static str {
        "moonshotai"
    }

    async fn fetch(&self, client: &Client) -> Result<ProviderUsage> {
        let url = format!(
            "{}/v1/users/me/balance",
            self.base_url.trim_end_matches('/')
        );
        let body = get_json(
            client,
            &url,
            &[("Authorization", format!("Bearer {}", self.api_key))],
        )
        .await?;
        let data = &body["data"];
        let available = data["available_balance"].as_f64();
        let voucher = data["voucher_balance"].as_f64();
        let cash = data["cash_balance"].as_f64();
        Ok(ProviderUsage {
            provider: "moonshotai".to_string(),
            plan: None,
            used: None,
            limit: None,
            remaining: available,
            unit: Some("balance".to_string()),
            resets_at: None,
            detail: match (voucher, cash) {
                (Some(voucher), Some(cash)) => {
                    Some(format!("voucher {voucher:.2}, cash {cash:.2}"))
                }
                _ => None,
            },
            source: url,
            fetched_at_ms: now_ms(),
            cache_age_secs: None,
        })
    }
}

/// Reads GitHub Copilot entitlement and limited-user quota information.
pub struct CopilotUsageReader {
    github_token: String,
    base_url: String,
}

impl CopilotUsageReader {
    /// Construct a reader using the production GitHub API endpoint.
    #[must_use]
    pub fn new(github_token: String) -> Self {
        Self::with_base_url(github_token, "https://api.github.com".to_string())
    }

    /// Construct a reader with an explicit base URL, primarily for local tests.
    #[must_use]
    pub const fn with_base_url(github_token: String, base_url: String) -> Self {
        Self {
            github_token,
            base_url,
        }
    }
}

#[async_trait::async_trait]
impl UsageReader for CopilotUsageReader {
    fn provider(&self) -> &'static str {
        "github-copilot"
    }

    async fn fetch(&self, client: &Client) -> Result<ProviderUsage> {
        let url = format!(
            "{}/copilot_internal/v2/token",
            self.base_url.trim_end_matches('/')
        );
        let body = get_json(
            client,
            &url,
            &[
                ("Authorization", format!("token {}", self.github_token)),
                ("User-Agent", "pi-agent-rust".to_string()),
            ],
        )
        .await?;
        let plan = body["sku"].as_str().map(str::to_string);
        let chat_quota = body["limited_user_quotas"]["chat"].as_f64();
        let reset = body["limited_user_reset_date"].as_str().map(str::to_string);
        let completions = body["limited_user_quotas"]["completions"].as_f64();
        Ok(ProviderUsage {
            provider: "github-copilot".to_string(),
            plan,
            used: None,
            limit: None,
            remaining: chat_quota,
            unit: chat_quota.map(|_| "chat requests".to_string()),
            resets_at: reset,
            detail: completions.map(|value| format!("completions quota {value}")),
            source: url,
            fetched_at_ms: now_ms(),
            cache_age_secs: None,
        })
    }
}

/// Perform a bounded JSON GET without exposing response bodies in errors.
async fn get_json(
    client: &Client,
    url: &str,
    headers: &[(&str, String)],
) -> Result<serde_json::Value> {
    let mut request = client.get(url).timeout(USAGE_FETCH_TIMEOUT);
    for (name, value) in headers {
        request = request.header(*name, value.clone());
    }
    let response = Box::pin(request.send()).await?;
    let status = response.status();
    if !(200..300).contains(&status) {
        // Provider bodies can contain account identifiers or diagnostic data;
        // status-only errors keep those fields out of CLI and transcript output.
        return Err(Error::api(format!("usage endpoint returned HTTP {status}")));
    }
    let text = response.text().await?;
    serde_json::from_str(&text)
        .map_err(|_| Error::api("usage endpoint returned invalid JSON".to_string()))
}

const NO_ENDPOINT_REASON: &[(&str, &str)] = &[
    (
        "anthropic",
        "no public quota endpoint (rate-limit state arrives only in response headers)",
    ),
    (
        "openai",
        "no public quota endpoint (usage dashboard requires a browser session)",
    ),
];

/// Readers plus documented-unavailable rows for configured providers.
pub type ConfiguredReaders = (Vec<Box<dyn UsageReader>>, Vec<(String, String)>);

/// Build quota readers from the current auth storage.
#[must_use]
pub fn readers_from_auth(auth: &AuthStorage) -> ConfiguredReaders {
    let mut readers: Vec<Box<dyn UsageReader>> = Vec::new();
    let mut unavailable = Vec::new();

    if let Some(key) = auth.resolve_api_key("openrouter", None) {
        readers.push(Box::new(OpenRouterUsageReader::new(key)));
    }
    if let Some(key) = auth.resolve_api_key("moonshotai", None) {
        readers.push(Box::new(MoonshotUsageReader::new(key)));
    }
    if let Some(token) = auth.resolve_api_key("github-copilot", None) {
        readers.push(Box::new(CopilotUsageReader::new(token)));
    }
    for (provider, reason) in NO_ENDPOINT_REASON {
        if auth.resolve_api_key(provider, None).is_some() {
            unavailable.push(((*provider).to_string(), (*reason).to_string()));
        }
    }

    (readers, unavailable)
}

type CachedUsage = (Instant, ProviderUsage);
static USAGE_CACHE: Mutex<Option<HashMap<String, CachedUsage>>> = Mutex::new(None);

fn cache_get(provider: &str, max_age: Duration) -> Option<(Duration, ProviderUsage)> {
    let guard = match USAGE_CACHE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let cache = guard.as_ref()?;
    let (stored_at, usage) = cache.get(provider)?;
    let age = stored_at.elapsed();
    (age <= max_age).then(|| (age, usage.clone()))
}

fn cache_put(provider: &str, usage: &ProviderUsage) {
    let mut guard = match USAGE_CACHE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(provider.to_string(), (Instant::now(), usage.clone()));
}

/// Gather usage for every provider with a supported or documented endpoint.
///
/// Fresh rows are served from memory unless `refresh` is true. A failed live
/// read uses an older cached row with an age marker; only providers without a
/// cached result become `Error` rows.
pub async fn gather_usage(auth: &AuthStorage, refresh: bool) -> Vec<UsageStatus> {
    let (readers, unavailable) = readers_from_auth(auth);
    gather_usage_from_readers(readers, unavailable, refresh).await
}

async fn gather_usage_from_readers(
    readers: Vec<Box<dyn UsageReader>>,
    unavailable: Vec<(String, String)>,
    refresh: bool,
) -> Vec<UsageStatus> {
    let client = Client::new();
    let mut rows = Vec::with_capacity(readers.len() + unavailable.len());

    for reader in readers {
        let provider = reader.provider().to_string();
        if !refresh && let Some((age, mut usage)) = cache_get(&provider, USAGE_CACHE_TTL) {
            usage.cache_age_secs = Some(age.as_secs());
            rows.push(UsageStatus::Ready(usage));
            continue;
        }

        let fetched = asupersync::time::timeout(
            asupersync::time::wall_now(),
            USAGE_FETCH_TIMEOUT,
            reader.fetch(&client),
        )
        .await;
        match fetched {
            Ok(Ok(usage)) => {
                cache_put(&provider, &usage);
                rows.push(UsageStatus::Ready(usage));
            }
            Ok(Err(error)) => rows.push(stale_or_error(&provider, &error.to_string())),
            Err(_) => rows.push(stale_or_error(
                &provider,
                &format!("timed out after {}s", USAGE_FETCH_TIMEOUT.as_secs()),
            )),
        }
    }

    rows.extend(
        unavailable
            .into_iter()
            .map(|(provider, reason)| UsageStatus::Unavailable { provider, reason }),
    );
    rows
}

fn stale_or_error(provider: &str, error: &str) -> UsageStatus {
    cache_get(provider, Duration::MAX).map_or_else(
        || UsageStatus::Error {
            provider: provider.to_string(),
            error: error.to_string(),
        },
        |(age, mut usage)| {
            usage.cache_age_secs = Some(age.as_secs());
            let previous_detail = usage.detail.take();
            usage.detail = Some(match previous_detail {
                Some(detail) => format!("{detail}; live read failed: {error}"),
                None => format!("live read failed: {error}"),
            });
            UsageStatus::Ready(usage)
        },
    )
}

/// Render usage rows for terminals and interactive transcripts.
#[must_use]
pub fn render_usage_text(rows: &[UsageStatus]) -> String {
    if rows.is_empty() {
        return "No providers with credentials configured. Run /login <provider> first."
            .to_string();
    }

    let mut lines = vec!["Provider usage:".to_string()];
    for row in rows {
        match row {
            UsageStatus::Ready(usage) => {
                let mut parts = Vec::new();
                if let Some(plan) = &usage.plan {
                    parts.push(format!("plan {plan}"));
                }
                match (usage.used, usage.limit) {
                    (Some(used), Some(limit)) => {
                        parts.push(format!("{used:.2} of {limit:.2} used"));
                    }
                    (Some(used), None) => parts.push(format!("{used:.2} used")),
                    _ => {}
                }
                if let Some(remaining) = usage.remaining {
                    parts.push(format!("{remaining:.2} remaining"));
                }
                if let Some(unit) = &usage.unit {
                    parts.push(format!("({unit})"));
                }
                if let Some(resets_at) = &usage.resets_at {
                    parts.push(format!("resets {resets_at}"));
                }
                if let Some(detail) = &usage.detail {
                    parts.push(format!("— {detail}"));
                }
                if let Some(age) = usage.cache_age_secs {
                    parts.push(format!("[cached {age}s ago]"));
                }
                if parts.is_empty() {
                    parts.push("no quota data in response".to_string());
                }
                lines.push(format!("  {}: {}", usage.provider, parts.join(" ")));
            }
            UsageStatus::Unavailable { provider, reason } => {
                lines.push(format!("  {provider}: unavailable — {reason}"));
            }
            UsageStatus::Error { provider, error } => {
                lines.push(format!("  {provider}: read failed — {error}"));
            }
        }
    }
    lines.join("\n")
}

/// Render usage rows as schema-tagged JSON.
#[must_use]
pub fn render_usage_json(rows: &[UsageStatus]) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": USAGE_SCHEMA,
        "providers": rows,
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn run_async<F: std::future::Future>(future: F) -> F::Output {
        asupersync::runtime::RuntimeBuilder::current_thread()
            .build()
            .expect("test runtime")
            .block_on(future)
    }

    fn spawn_json_server(status: u16, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener");
        let address = listener.local_addr().expect("test listener address");
        let body = body.to_string();
        std::thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut request = [0_u8; 4096];
                let _ = socket.read(&mut request);
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes());
            }
        });
        format!("http://{address}")
    }

    #[test]
    fn openrouter_reader_parses_credits() {
        let base = spawn_json_server(200, r#"{"data":{"total_credits":25.0,"total_usage":10.5}}"#);
        let reader = OpenRouterUsageReader::with_base_url("test-key".to_string(), base);
        let usage = run_async(async { reader.fetch(&Client::new()).await.expect("credits") });
        assert_eq!(usage.limit, Some(25.0));
        assert_eq!(usage.used, Some(10.5));
        assert_eq!(usage.remaining, Some(14.5));
    }

    #[test]
    fn moonshot_reader_parses_balance() {
        let base = spawn_json_server(
            200,
            r#"{"data":{"available_balance":42.5,"voucher_balance":2.5,"cash_balance":40.0}}"#,
        );
        let reader = MoonshotUsageReader::with_base_url("test-key".to_string(), base);
        let usage = run_async(async { reader.fetch(&Client::new()).await.expect("balance") });
        assert_eq!(usage.provider, "moonshotai");
        assert_eq!(usage.remaining, Some(42.5));
        assert!(
            usage
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("voucher 2.50"))
        );
    }

    #[test]
    fn copilot_reader_parses_entitlement_and_quota() {
        let base = spawn_json_server(
            200,
            r#"{"sku":"free_limited_copilot","limited_user_quotas":{"chat":32.0,"completions":100.0},"limited_user_reset_date":"2026-09-01"}"#,
        );
        let reader = CopilotUsageReader::with_base_url("test-token".to_string(), base);
        let usage = run_async(async { reader.fetch(&Client::new()).await.expect("copilot") });
        assert_eq!(usage.plan.as_deref(), Some("free_limited_copilot"));
        assert_eq!(usage.remaining, Some(32.0));
        assert_eq!(usage.resets_at.as_deref(), Some("2026-09-01"));
    }

    #[test]
    fn reader_errors_are_status_only_and_do_not_include_body() {
        let base = spawn_json_server(401, r#"{"secret":"must-not-be-rendered"}"#);
        let reader = OpenRouterUsageReader::with_base_url("test-key".to_string(), base);
        let error = run_async(async { reader.fetch(&Client::new()).await.expect_err("401") });
        let message = error.to_string();
        assert!(message.contains("HTTP 401"), "{message}");
        assert!(!message.contains("must-not-be-rendered"), "{message}");
    }

    struct TestReader {
        provider: &'static str,
        calls: Arc<AtomicUsize>,
        should_fail: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl UsageReader for TestReader {
        fn provider(&self) -> &'static str {
            self.provider
        }

        async fn fetch(&self, _client: &Client) -> Result<ProviderUsage> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.should_fail.load(Ordering::SeqCst) {
                return Err(Error::api("test read failed"));
            }
            Ok(ProviderUsage {
                provider: self.provider.to_string(),
                plan: None,
                used: Some(1.0),
                limit: Some(2.0),
                remaining: Some(1.0),
                unit: Some("test units".to_string()),
                resets_at: None,
                detail: None,
                source: "test".to_string(),
                fetched_at_ms: now_ms(),
                cache_age_secs: None,
            })
        }
    }

    fn test_reader(provider: &'static str) -> (TestReader, Arc<AtomicUsize>, Arc<AtomicBool>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let should_fail = Arc::new(AtomicBool::new(false));
        (
            TestReader {
                provider,
                calls: Arc::clone(&calls),
                should_fail: Arc::clone(&should_fail),
            },
            calls,
            should_fail,
        )
    }

    #[test]
    fn readers_from_auth_classifies_unavailable_and_aliases() {
        let path =
            std::env::temp_dir().join(format!("pi_usage_auth_test_{}.json", std::process::id()));
        let mut auth = AuthStorage::load(path).expect("auth storage");
        auth.set(
            "anthropic",
            crate::auth::AuthCredential::ApiKey {
                key: "anthropic-test-key".to_string(),
            },
        );
        auth.set(
            "kimi",
            crate::auth::AuthCredential::ApiKey {
                key: "kimi-test-key".to_string(),
            },
        );

        let (readers, unavailable) = readers_from_auth(&auth);
        assert_eq!(readers.len(), 1);
        assert_eq!(readers[0].provider(), "moonshotai");
        assert!(unavailable.iter().any(|(provider, reason)| {
            provider == "anthropic" && reason.contains("no public quota endpoint")
        }));
    }

    #[test]
    fn gather_uses_cache_refresh_and_stale_fallback() {
        let (reader, calls, should_fail) = test_reader("test-cache-refresh");
        let first = run_async(gather_usage_from_readers(
            vec![Box::new(reader)],
            Vec::new(),
            false,
        ));
        assert!(matches!(first.as_slice(), [UsageStatus::Ready(_)]));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let (reader, _, _) = test_reader("test-cache-refresh");
        let cached = run_async(gather_usage_from_readers(
            vec![Box::new(reader)],
            Vec::new(),
            false,
        ));
        assert!(matches!(
            cached.as_slice(),
            [UsageStatus::Ready(ProviderUsage {
                cache_age_secs: Some(_),
                ..
            })]
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let (reader, _, _) = test_reader("test-cache-refresh");
        let refreshed = run_async(gather_usage_from_readers(
            vec![Box::new(reader)],
            Vec::new(),
            true,
        ));
        assert!(matches!(refreshed.as_slice(), [UsageStatus::Ready(_)]));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        should_fail.store(true, Ordering::SeqCst);
        // The shared failure switch belongs to the first reader; use it in a
        // reader instance so the live failure can exercise the cached row.
        let stale_reader = TestReader {
            provider: "test-cache-refresh",
            calls: Arc::clone(&calls),
            should_fail: Arc::clone(&should_fail),
        };
        let stale = run_async(gather_usage_from_readers(
            vec![Box::new(stale_reader)],
            Vec::new(),
            true,
        ));
        assert!(matches!(
            stale.as_slice(),
            [UsageStatus::Ready(ProviderUsage {
                cache_age_secs: Some(_),
                detail: Some(detail),
                ..
            })] if detail.contains("live read failed")
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn gather_reports_error_without_cached_row() {
        let (reader, _, should_fail) = test_reader("test-no-cache-error");
        should_fail.store(true, Ordering::SeqCst);
        let rows = run_async(gather_usage_from_readers(
            vec![Box::new(reader)],
            Vec::new(),
            true,
        ));
        assert!(
            matches!(rows.as_slice(), [UsageStatus::Error { provider, error }] if provider == "test-no-cache-error" && error.contains("test read failed"))
        );
    }

    #[test]
    fn renderers_cover_ready_unavailable_and_error() {
        let rows = vec![
            UsageStatus::Ready(ProviderUsage {
                provider: "openrouter".to_string(),
                plan: None,
                used: Some(10.5),
                limit: Some(25.0),
                remaining: Some(14.5),
                unit: Some("USD credits".to_string()),
                resets_at: None,
                detail: None,
                source: "test".to_string(),
                fetched_at_ms: 0,
                cache_age_secs: Some(30),
            }),
            UsageStatus::Unavailable {
                provider: "anthropic".to_string(),
                reason: "no public quota endpoint".to_string(),
            },
            UsageStatus::Error {
                provider: "moonshotai".to_string(),
                error: "timed out".to_string(),
            },
        ];
        let text = render_usage_text(&rows);
        assert!(text.contains("10.50 of 25.00 used"), "{text}");
        assert!(text.contains("[cached 30s ago]"), "{text}");
        assert!(text.contains("anthropic: unavailable"), "{text}");
        assert!(text.contains("moonshotai: read failed"), "{text}");

        let json = render_usage_json(&rows);
        let value: serde_json::Value = serde_json::from_str(&json).expect("usage JSON");
        assert_eq!(value["schema"], USAGE_SCHEMA);
        assert_eq!(value["providers"][0]["status"], "ready");
        assert_eq!(value["providers"][1]["status"], "unavailable");
    }
}
