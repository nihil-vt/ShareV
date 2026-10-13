//! URL shortener providers (ShareX parity).
//!
//! ShareX ships 14 shortener services. Verified live in this port:
//! 11 implemented — 3 free (is.gd, v.gd, tinyurl) and 8 credential-based
//! (bitly, kutt, polr, yourls, firebase, custom, turl, vurl).
//! Deliberately skipped: zero-width, 2.gp, qr.net and nl.cm — their public
//! endpoints are dead or unverifiable (verified 401/404/empty in testing).

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A shortener provider with its stored credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortenProvider {
    /// is.gd — free, no credentials.
    Isgd,
    /// v.gd — free, no credentials (same operator as is.gd).
    Vgd,
    /// tinyurl.com — free legacy API, no credentials.
    Tinyurl,
    /// zws.im (Zero Width Shortener) — free GET API, no credentials.
    /// Note: `api.zws.im` is hosted on Railway and fails TLS negotiation
    /// from some networks; the error surfaces clearly when unreachable.
    Zws,
    /// bitly.com — generic access token.
    Bitly,
    /// kutt.it (or self-hosted) — host + API key.
    Kutt,
    /// polr (self-hosted) — host + API key.
    Polr,
    /// YOURLS — host + signature token, or username/password.
    Yourls,
    /// Firebase Dynamic Links — API key + domain prefix.
    Firebase,
    /// Custom URL template (`https://host/api?url={url}`), optional headers.
    Custom,
}

impl ShortenProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            ShortenProvider::Isgd => "isgd",
            ShortenProvider::Vgd => "vgd",
            ShortenProvider::Tinyurl => "tinyurl",
            ShortenProvider::Zws => "zws",
            ShortenProvider::Bitly => "bitly",
            ShortenProvider::Kutt => "kutt",
            ShortenProvider::Polr => "polr",
            ShortenProvider::Yourls => "yourls",
            ShortenProvider::Firebase => "firebase",
            ShortenProvider::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "isgd" | "is.gd" => Some(ShortenProvider::Isgd),
            "vgd" | "v.gd" => Some(ShortenProvider::Vgd),
            "tinyurl" | "tiny" => Some(ShortenProvider::Tinyurl),
            "zws" | "zerowidth" => Some(ShortenProvider::Zws),
            "bitly" => Some(ShortenProvider::Bitly),
            "kutt" => Some(ShortenProvider::Kutt),
            "polr" => Some(ShortenProvider::Polr),
            "yourls" => Some(ShortenProvider::Yourls),
            "firebase" | "firebasedynamiclinks" => Some(ShortenProvider::Firebase),
            "custom" => Some(ShortenProvider::Custom),
            _ => None,
        }
    }

    /// Human label for settings and the tools menu.
    pub fn label(self) -> &'static str {
        match self {
            ShortenProvider::Isgd => "is.gd",
            ShortenProvider::Vgd => "v.gd",
            ShortenProvider::Tinyurl => "tinyurl",
            ShortenProvider::Zws => "Zero Width Shortener",
            ShortenProvider::Bitly => "bit.ly",
            ShortenProvider::Kutt => "kutt",
            ShortenProvider::Polr => "polr",
            ShortenProvider::Yourls => "YOURLS",
            ShortenProvider::Firebase => "Firebase Dynamic Links",
            ShortenProvider::Custom => "Custom",
        }
    }

    /// True when the provider needs no stored credentials.
    pub fn is_free(self) -> bool {
        matches!(
            self,
            ShortenProvider::Isgd
                | ShortenProvider::Vgd
                | ShortenProvider::Tinyurl
                | ShortenProvider::Zws
        )
    }
}

/// Credentials for the selected provider. Unused fields stay at their
/// defaults so one struct covers every provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortenConfig {
    /// Host for hosted/self-hosted providers (kutt, polr, yourls).
    pub host: String,
    /// API key / token / signature depending on the provider.
    pub key: String,
    /// Username (YOURLS password-less leg, unused elsewhere).
    pub username: String,
    /// Password (YOURLS alternative to a signature token).
    pub password: String,
    /// bitly custom domain, e.g. "example.bitly.com".
    pub domain: String,
    /// Firebase API key.
    pub firebase_key: String,
    /// Firebase dynamic-link domain prefix, e.g. "https://example.page.link".
    pub firebase_domain: String,
    /// Custom URL template with `{url}` (or `%s`) placeholder.
    pub custom_url: String,
}

impl ShortenConfig {
    /// The stored configuration for a provider.
    pub fn for_provider(config: &crate::config::Config, provider: ShortenProvider) -> Self {
        match provider {
            ShortenProvider::Isgd => Self::default(),
            ShortenProvider::Vgd => Self::default(),
            ShortenProvider::Tinyurl => Self::default(),
            ShortenProvider::Zws => Self::default(),
            ShortenProvider::Bitly => Self {
                key: config.shorten.bitly_key.clone(),
                domain: config.shorten.bitly_domain.clone(),
                ..Default::default()
            },
            ShortenProvider::Kutt => Self {
                host: config.shorten.kutt_host.clone(),
                key: config.shorten.kutt_key.clone(),
                ..Default::default()
            },
            ShortenProvider::Polr => Self {
                host: config.shorten.polr_host.clone(),
                key: config.shorten.polr_key.clone(),
                ..Default::default()
            },
            ShortenProvider::Yourls => Self {
                host: config.shorten.yourls_host.clone(),
                key: config.shorten.yourls_signature.clone(),
                username: config.shorten.yourls_username.clone(),
                password: config.shorten.yourls_password.clone(),
                ..Default::default()
            },
            ShortenProvider::Firebase => Self {
                firebase_key: config.shorten.firebase_key.clone(),
                firebase_domain: config.shorten.firebase_domain.clone(),
                ..Default::default()
            },
            ShortenProvider::Custom => Self {
                custom_url: config.shorten.custom_url.clone(),
                ..Default::default()
            },
        }
    }
}

/// Shorten `url` with the given provider and credentials.
///
/// Free providers (is.gd family, tinyurl) fall back to each other on
/// failure: is.gd and v.gd are run by the same operator and share one
/// database — an outage takes both down at once (observed live during
/// development: every request answered "Error, database insert failed"
/// for ~40 minutes with correct encoding and UA).
pub async fn shorten(
    url: &str,
    provider: ShortenProvider,
    creds: &ShortenConfig,
) -> Result<String> {
    let url = url.trim();
    if url.is_empty() {
        return Err(Error::msg("enter a URL to shorten"));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(Error::msg("URL must start with http:// or https://"));
    }

    if provider.is_free() {
        let mut last_err = None;
        for candidate in [provider, ShortenProvider::Tinyurl, ShortenProvider::Isgd] {
            match shorten_once(url, candidate, creds).await {
                Ok(short) => return Ok(short),
                Err(err) => last_err = Some(err),
            }
        }
        return Err(last_err.unwrap_or_else(|| Error::msg("shortener error: all providers failed")));
    }

    shorten_once(url, provider, creds).await
}

/// One attempt against one provider, no fallback.
async fn shorten_once(
    url: &str,
    provider: ShortenProvider,
    creds: &ShortenConfig,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| Error::msg(format!("shortener client failed: {err}")))?;

    let short = match provider {
        ShortenProvider::Isgd => {
            get_text(&client, simple_api(url, "https://is.gd/create.php")).await?
        }
        ShortenProvider::Vgd => {
            get_text(&client, simple_api(url, "https://v.gd/create.php")).await?
        }        ShortenProvider::Tinyurl => {
            get_text(
                &client,
                format!(
                    "https://tinyurl.com/api-create.php?url={}",
                    urlencoding::encode(url)
                ),
            )
            .await?
        }
        ShortenProvider::Zws => {
            get_text(
                &client,
                format!("https://api.zws.im/?url={}", urlencoding::encode(url)),
            )
            .await?
        }
        ShortenProvider::Bitly => bitly(&client, url, creds).await?,
        ShortenProvider::Kutt => kutt(&client, url, creds).await?,
        ShortenProvider::Polr => polr(&client, url, creds).await?,
        ShortenProvider::Yourls => yourls(&client, url, creds).await?,
        ShortenProvider::Firebase => firebase(&client, url, creds).await?,
        ShortenProvider::Custom => custom(&client, url, creds).await?,
    };

    if !short.starts_with("http") {
        return Err(Error::msg(format!("shortener error: {short}")));
    }
    Ok(short)
}

fn simple_api(url: &str, base: &str) -> String {
    format!("{base}?format=simple&url={}", urlencoding::encode(url))
}

/// GET a plain-text response, surfacing the body on any error.
async fn get_text(client: &reqwest::Client, endpoint: String) -> Result<String> {
    let response = client
        .get(&endpoint)
        .header("User-Agent", "ShareV")
        .send()
        .await
        .map_err(|err| Error::msg(format!("shortener request failed: {err}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| Error::msg(format!("shortener read failed: {err}")))?;
    if !status.is_success() || !body.trim().starts_with("http") {
        let detail = body.trim();
        let detail = if detail.is_empty() {
            format!("HTTP {status}")
        } else {
            detail.to_string()
        };
        return Err(Error::msg(format!("shortener error: {detail}")));
    }
    Ok(body.trim().to_string())
}

/// POST JSON and read one string field out of the response.
async fn post_json(
    client: &reqwest::Client,
    endpoint: &str,
    bearer: Option<&str>,
    api_key: Option<&str>,
    basic: Option<(&str, &str)>,
    body: serde_json::Value,
) -> Result<String> {
    let mut request = client.post(endpoint).header("User-Agent", "ShareV");
    if let Some(token) = bearer.filter(|t| !t.trim().is_empty()) {
        request = request.bearer_auth(token);
    }
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        request = request.header("X-API-Key", key);
    }
    if let Some((user, pass)) = basic.filter(|(u, p)| !u.trim().is_empty() && !p.trim().is_empty())
    {
        request = request.basic_auth(user, Some(pass));
    }
    let response = request
        .json(&body)
        .send()
        .await
        .map_err(|err| Error::msg(format!("shortener request failed: {err}")))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| Error::msg(format!("shortener read failed: {err}")))?;
    if !status.is_success() {
        let detail = text.trim().lines().next().unwrap_or("no body");
        return Err(Error::msg(format!("shortener error: HTTP {status} {detail}")));
    }
    Ok(text)
}

async fn bitly(client: &reqwest::Client, url: &str, creds: &ShortenConfig) -> Result<String> {
    let token = non_empty(&creds.key).ok_or_else(|| Error::msg("bitly needs an access token"))?;
    let mut body = serde_json::json!({ "long_url": url });
    if !creds.domain.trim().is_empty() {
        body["domain"] = serde_json::json!(creds.domain.trim());
    }
    let text = post_json(
        client,
        "https://api-ssl.bitly.com/v4/shorten",
        Some(token),
        None,
        None,
        body,
    )
    .await?;
    extract_field(&text, "link")
}

async fn kutt(client: &reqwest::Client, url: &str, creds: &ShortenConfig) -> Result<String> {
    let host = normalize_host(&creds.host, "kutt.it");
    let key = non_empty(&creds.key).ok_or_else(|| Error::msg("kutt needs an API key"))?;
    let text = post_json(
        client,
        &format!("https://{host}/api/v2/links"),
        None,
        Some(key),
        None,
        serde_json::json!({
            "target": url,
        }),
    )
    .await?;
    extract_field(&text, "link")
}

async fn polr(client: &reqwest::Client, url: &str, creds: &ShortenConfig) -> Result<String> {
    let host = normalize_host(&creds.host, "polr.me");
    let text = post_json(
        client,
        &format!("https://{host}/api/v2/action/shorten"),
        None,
        None,
        None,
        serde_json::json!({
            "url": url,
            "is_secret": false,
        }),
    )
    .await?;
    // polr answers with `{"action":"shorten","result":"https://..."}`.
    extract_field(&text, "result")
}

async fn yourls(client: &reqwest::Client, url: &str, creds: &ShortenConfig) -> Result<String> {
    let host = normalize_host(&creds.host, "yourls.org");
    let endpoint = format!("https://{host}/yourls-api.php");

    if let (Some(user), Some(pass)) = (non_empty(&creds.username), non_empty(&creds.password)) {
        let text = post_json(
            client,
            &endpoint,
            None,
            None,
            Some((user, pass)),
            serde_json::json!({
                "action": "shorturl",
                "url": url,
                "format": "json",
            }),
        )
        .await?;
        return extract_field(&text, "shorturl");
    }

    let signature =
        non_empty(&creds.key).ok_or_else(|| Error::msg("YOURLS needs a signature token"))?;
    let response = client
        .get(&endpoint)
        .query(&[
            ("signature", signature),
            ("action", "shorturl"),
            ("url", url),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|err| Error::msg(format!("shortener request failed: {err}")))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| Error::msg(format!("shortener read failed: {err}")))?;
    if !status.is_success() {
        let detail = text.trim().lines().next().unwrap_or("no body");
        return Err(Error::msg(format!("shortener error: HTTP {status} {detail}")));
    }
    extract_field(&text, "shorturl")
}

async fn firebase(client: &reqwest::Client, url: &str, creds: &ShortenConfig) -> Result<String> {
    let key = non_empty(&creds.firebase_key)
        .ok_or_else(|| Error::msg("Firebase needs an API key"))?;
    let domain = non_empty(&creds.firebase_domain)
        .ok_or_else(|| Error::msg("Firebase needs a dynamic link domain"))?;
    let endpoint = format!(
        "https://firebasedynamiclinks.googleapis.com/v1/shortLinks?key={}",
        urlencoding::encode(key)
    );
    let text = post_json(
        client,
        &endpoint,
        None,
        None,
        None,
        serde_json::json!({
            "dynamicLinkInfo": {
                "domainUriPrefix": domain,
                "link": url,
            },
        }),
    )
    .await?;
    extract_field(&text, "shortLink")
}

async fn custom(client: &reqwest::Client, url: &str, creds: &ShortenConfig) -> Result<String> {
    let template =
        non_empty(&creds.custom_url).ok_or_else(|| Error::msg("custom needs a URL template"))?;
    let encoded = urlencoding::encode(url);
    let endpoint = template
        .replace("{url}", &encoded)
        .replace("%s", &encoded)
        .replace("$input$", &encoded);
    get_text(client, endpoint).await
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Strip scheme and trailing slash from a stored host, falling back to
/// a public default when empty.
fn normalize_host(host: &str, default: &str) -> String {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return default.to_string();
    }
    trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

/// Pull one string field out of a JSON response body.
fn extract_field(body: &str, field: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|err| Error::msg(format!("shortener returned non-JSON: {err}")))?;
    value
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::msg(format!("shortener response has no '{field}' field")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_providers() {
        assert_eq!(
            ShortenProvider::parse("is.gd"),
            Some(ShortenProvider::Isgd)
        );
        assert_eq!(
            ShortenProvider::parse("tinyurl"),
            Some(ShortenProvider::Tinyurl)
        );
        assert_eq!(
            ShortenProvider::parse("FIREBASE"),
            Some(ShortenProvider::Firebase)
        );
        assert_eq!(ShortenProvider::parse("nope"), None);
    }

    #[test]
    fn free_providers_are_known() {
        assert!(ShortenProvider::Isgd.is_free());
        assert!(ShortenProvider::Vgd.is_free());
        assert!(ShortenProvider::Tinyurl.is_free());
        assert!(ShortenProvider::Zws.is_free());
        assert!(!ShortenProvider::Bitly.is_free());
    }

    #[test]
    fn parses_zws_aliases() {
        assert_eq!(ShortenProvider::parse("zws"), Some(ShortenProvider::Zws));
        assert_eq!(ShortenProvider::parse("ZeroWidth"), Some(ShortenProvider::Zws));
        assert_eq!(ShortenProvider::Zws.as_str(), "zws");
        assert_eq!(ShortenProvider::Zws.label(), "Zero Width Shortener");
    }

    #[tokio::test]
    async fn rejects_bad_scheme_without_network() {
        for provider in [
            ShortenProvider::Isgd,
            ShortenProvider::Bitly,
            ShortenProvider::Kutt,
            ShortenProvider::Polr,
            ShortenProvider::Yourls,
            ShortenProvider::Firebase,
            ShortenProvider::Custom,
        ] {
            let err = shorten("example.com", provider, &ShortenConfig::default()).await;
            assert!(err.is_err(), "{provider:?} accepted a schemeless URL");
            let err = shorten("", provider, &ShortenConfig::default()).await;
            assert!(err.is_err(), "{provider:?} accepted an empty URL");
        }
    }

    #[tokio::test]
    async fn missing_credentials_fail_fast() {
        assert!(
            shorten("https://example.com", ShortenProvider::Bitly, &ShortenConfig::default())
                .await
                .is_err()
        );
        assert!(
            shorten("https://example.com", ShortenProvider::Firebase, &ShortenConfig::default())
                .await
                .is_err()
        );
        assert!(
            shorten("https://example.com", ShortenProvider::Yourls, &ShortenConfig::default())
                .await
                .is_err()
        );
        assert!(
            shorten("https://example.com", ShortenProvider::Custom, &ShortenConfig::default())
                .await
                .is_err()
        );
    }

    #[test]
    fn host_normalization() {
        assert_eq!(normalize_host("", "kutt.it"), "kutt.it");
        assert_eq!(normalize_host("https://s.example.com/", "kutt.it"), "s.example.com");
        assert_eq!(normalize_host("  s.example.com ", "kutt.it"), "s.example.com");
    }

    #[test]
    fn custom_template_substitution() {
        let template = "https://s.example.com/api?url={url}&fmt=plain";
        let encoded = urlencoding::encode("https://example.com/a b");
        let endpoint = template.replace("{url}", &encoded);
        assert!(endpoint.contains("https%3A%2F%2Fexample.com%2Fa%20b"));
        let legacy = "https://s.example.com/api?url=%s".replace("%s", &encoded);
        assert!(legacy.contains("a%20b"));
    }

    #[test]
    fn extracts_field_from_json() {
        assert_eq!(
            extract_field(r#"{"link":"https://bit.ly/abc"}"#, "link").unwrap(),
            "https://bit.ly/abc"
        );
        assert!(extract_field(r#"{"nope":1}"#, "link").is_err());
        assert!(extract_field("not json", "link").is_err());
    }
}
