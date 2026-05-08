#![allow(missing_docs)]

use std::env;

use crate::models::manifestation_format::ManifestationFormat;

// Env-var lookup function. `Config::from_env` reads from process env; tests
// inject a `HashMap`-backed closure via `Config::from_source` so test setup
// never mutates global state. UNK-100; lifts
// `debt/2026-05-05-env-lock-config-tests.md`.
type EnvGet<'a> = dyn Fn(&str) -> Option<String> + 'a;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub library_path: String,
    pub ingestion_path: String,
    pub quarantine_path: String,
    pub log_level: String,
    pub db_max_connections: u32,
    pub oidc_issuer_url: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_redirect_uri: String,
    pub ingestion_database_url: String,
    pub format_priority: Vec<ManifestationFormat>,
    pub cleanup_mode: CleanupMode,
    pub enrichment: EnrichmentConfig,
    pub cover: CoverConfig,
    pub writeback: WritebackConfig,
    pub opds: OpdsConfig,
    pub security: SecurityConfig,
    pub openlibrary_base_url: String,
    pub googlebooks_base_url: String,
    pub googlebooks_api_key: Option<String>,
    pub hardcover_base_url: String,
    pub hardcover_api_token: Option<String>,
    pub operator_contact: Option<String>,
}

/// OPDS catalog configuration. When `enabled`, `/opds/*` is mounted behind a
/// Basic-only extractor and `public_url` must be set — feeds emit absolute URLs
/// rooted at `public_url`.
///
/// Note: the dual-mounted cover handlers at `/api/books/:id/cover{,/thumb}` are
/// mounted independently of `enabled` because the web UI (Step 10) needs them
/// regardless of OPDS availability.
#[derive(Debug, Clone)]
pub struct OpdsConfig {
    pub enabled: bool,
    pub page_size: u32,
    pub realm: String,
    pub public_url: Option<url::Url>,
}

#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    pub enabled: bool,
    pub concurrency: u32,
    pub poll_idle_secs: u64,
    pub fetch_budget_secs: u64,
    pub http_timeout_secs: u64,
    pub max_attempts: u32,
    pub cache_ttl_hit_days: u32,
    pub cache_ttl_miss_days: u32,
    pub cache_ttl_error_mins: u32,
}

#[derive(Debug, Clone)]
pub struct CoverConfig {
    pub max_bytes: u64,
    pub download_timeout_secs: u64,
    pub min_long_edge_px: u32,
    pub redirect_limit: usize,
}

#[derive(Debug, Clone)]
pub struct WritebackConfig {
    pub enabled: bool,
    pub concurrency: u32,
    pub poll_idle_secs: u64,
    pub max_attempts: u32,
}

/// Response-header policy.
///
/// CSP values are stored as precomputed `HeaderValue`s. They depend on
/// `validate_frontend_dist` reading the on-disk FOUC script to derive its
/// hash, so `csp_api_header` and `csp_html_header` are left as `None` after
/// `from_env()` and finalised by `main()` via
/// [`crate::security::csp::build_api_csp`] /
/// [`crate::security::csp::build_html_csp`]. A non-conformant CSP string
/// panics in `main()` rather than silently dropping the header at request
/// time.
///
/// HSTS and Reporting-Endpoints are derived from the booleans / URL stored
/// here via [`Self::hsts_header_value`] and
/// [`Self::reporting_endpoints_header_value`]. Both compose static-ASCII
/// strings from validated inputs and panic on the impossible case (a
/// programming invariant has been violated and we want to know).
///
/// A `SecurityConfig` obtained directly from `from_env()` — without the
/// CSP-finalisation pass — emits no `Content-Security-Policy` on either
/// route class (both fields stay `None`); HSTS and Reporting-Endpoints
/// are still applied because they are derived on demand.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub behind_https: bool,
    pub hsts_include_subdomains: bool,
    pub hsts_preload: bool,
    pub csp_report_endpoint: Option<url::Url>,
    pub frontend_dist_path: Option<std::path::PathBuf>,
    pub csp_html_header: Option<axum::http::HeaderValue>,
    pub csp_api_header: Option<axum::http::HeaderValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    /// Delete all files in the ingestion directory after a successful batch
    All,
    /// Delete only files that were actually ingested (selected by format priority)
    Ingested,
    /// Never delete source files — user handles cleanup manually
    None,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingVar(String),
    #[error("invalid value for {var}: {reason}")]
    Invalid { var: String, reason: String },
}

/// Process-env adapter for `Config::from_env`. Extracted from a closure so it
/// is callable from a test (no `unsafe { env::set_var }` needed — tests read
/// vars cargo always sets, like `CARGO_PKG_NAME`).
fn process_env_get(k: &str) -> Option<String> {
    env::var(k).ok()
}

impl Config {
    /// Public entry point for production: loads `.env` (best-effort) then
    /// reads from process env via `std::env::var`.
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Self::from_source(&process_env_get)
    }

    /// Inject env via a closure. Tests pass a `HashMap`-backed `&EnvGet` so
    /// they never mutate process env (UNK-100). Production calls this via
    /// `from_env` with the `std::env::var` adapter.
    #[allow(
        clippy::too_many_lines,
        reason = "Config::from_source threads ~15 independent env vars with error propagation; extracting would produce boilerplate without improving readability"
    )]
    pub fn from_source(get: &EnvGet<'_>) -> Result<Self, ConfigError> {
        let database_url =
            get("DATABASE_URL").ok_or_else(|| ConfigError::MissingVar("DATABASE_URL".into()))?;

        let port = get("REVERIE_PORT")
            .unwrap_or_else(|| "3000".into())
            .parse::<u16>()
            .map_err(|e| ConfigError::Invalid {
                var: "REVERIE_PORT".into(),
                reason: e.to_string(),
            })?;

        let oidc_issuer_url = get("OIDC_ISSUER_URL")
            .ok_or_else(|| ConfigError::MissingVar("OIDC_ISSUER_URL".into()))?;
        let oidc_client_id = get("OIDC_CLIENT_ID")
            .ok_or_else(|| ConfigError::MissingVar("OIDC_CLIENT_ID".into()))?;
        let oidc_client_secret = get("OIDC_CLIENT_SECRET")
            .ok_or_else(|| ConfigError::MissingVar("OIDC_CLIENT_SECRET".into()))?;
        let oidc_redirect_uri = get("OIDC_REDIRECT_URI")
            .ok_or_else(|| ConfigError::MissingVar("OIDC_REDIRECT_URI".into()))?;

        let ingestion_database_url =
            get("DATABASE_URL_INGESTION").unwrap_or_else(|| database_url.clone());

        let format_priority: Vec<ManifestationFormat> = get("REVERIE_FORMAT_PRIORITY")
            .unwrap_or_else(|| "epub,pdf,mobi,azw3,cbz,cbr".into())
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<ManifestationFormat>()
                    .map_err(|_| ConfigError::Invalid {
                        var: "REVERIE_FORMAT_PRIORITY".into(),
                        reason: format!(
                            "unsupported format '{s}'. Supported: epub, pdf, mobi, azw3, cbz, cbr"
                        ),
                    })
            })
            .collect::<Result<_, _>>()?;

        let cleanup_mode = match get("REVERIE_CLEANUP_MODE")
            .unwrap_or_else(|| "all".into())
            .to_lowercase()
            .as_str()
        {
            "all" => CleanupMode::All,
            "ingested" => CleanupMode::Ingested,
            "none" => CleanupMode::None,
            other => {
                return Err(ConfigError::Invalid {
                    var: "REVERIE_CLEANUP_MODE".into(),
                    reason: format!("expected 'all', 'ingested', or 'none', got '{other}'"),
                });
            }
        };

        let enrichment = EnrichmentConfig::from_source(get)?;
        let cover = CoverConfig::from_source(get)?;
        let writeback = WritebackConfig::from_source(get)?;
        let opds = OpdsConfig::from_source(get)?;
        let security = SecurityConfig::from_source(get)?;

        let openlibrary_base_url =
            get("REVERIE_OPENLIBRARY_BASE_URL").unwrap_or_else(|| "https://openlibrary.org".into());
        let googlebooks_base_url = get("REVERIE_GOOGLEBOOKS_BASE_URL")
            .unwrap_or_else(|| "https://www.googleapis.com/books/v1".into());
        let googlebooks_api_key = get("REVERIE_GOOGLEBOOKS_API_KEY").filter(|s| !s.is_empty());
        let hardcover_base_url = get("REVERIE_HARDCOVER_BASE_URL")
            .unwrap_or_else(|| "https://api.hardcover.app/v1/graphql".into());
        let hardcover_api_token = get("REVERIE_HARDCOVER_API_TOKEN").filter(|s| !s.is_empty());
        let operator_contact = get("REVERIE_OPERATOR_CONTACT").filter(|s| !s.is_empty());

        Ok(Self {
            port,
            database_url,
            library_path: get("REVERIE_LIBRARY_PATH").unwrap_or_else(|| "./library".into()),
            ingestion_path: get("REVERIE_INGESTION_PATH").unwrap_or_else(|| "./ingestion".into()),
            quarantine_path: get("REVERIE_QUARANTINE_PATH")
                .unwrap_or_else(|| "./quarantine".into()),
            log_level: get("RUST_LOG").unwrap_or_else(|| "info".into()),
            db_max_connections: get("REVERIE_DB_MAX_CONNECTIONS")
                .unwrap_or_else(|| "10".into())
                .parse::<u32>()
                .map_err(|e| ConfigError::Invalid {
                    var: "REVERIE_DB_MAX_CONNECTIONS".into(),
                    reason: e.to_string(),
                })?,
            oidc_issuer_url,
            oidc_client_id,
            oidc_client_secret,
            oidc_redirect_uri,
            ingestion_database_url,
            format_priority,
            cleanup_mode,
            enrichment,
            cover,
            writeback,
            opds,
            security,
            openlibrary_base_url,
            googlebooks_base_url,
            googlebooks_api_key,
            hardcover_base_url,
            hardcover_api_token,
            operator_contact,
        })
    }

    /// `User-Agent` string for outbound metadata API requests.  `OpenLibrary`
    /// grants identified requests a 3 req/s rate-limit tier (vs. 1 req/s
    /// anonymous) when a contact email or URL is present in the UA.
    pub fn user_agent(&self) -> String {
        self.operator_contact.as_deref().map_or_else(
            || format!("Reverie/{} (unidentified)", env!("CARGO_PKG_VERSION")),
            |contact| format!("Reverie/{} ({contact})", env!("CARGO_PKG_VERSION")),
        )
    }
}

impl EnrichmentConfig {
    fn from_source(get: &EnvGet<'_>) -> Result<Self, ConfigError> {
        let enabled = parse_bool(get, "REVERIE_ENRICHMENT_ENABLED", true)?;
        let concurrency = parse_u32(get, "REVERIE_ENRICHMENT_CONCURRENCY", 2)?;
        if !(1..=10).contains(&concurrency) {
            return Err(ConfigError::Invalid {
                var: "REVERIE_ENRICHMENT_CONCURRENCY".into(),
                reason: format!("must be 1-10, got {concurrency}"),
            });
        }
        let poll_idle_secs = parse_u64(get, "REVERIE_ENRICHMENT_POLL_IDLE_SECS", 30)?;
        let fetch_budget_secs = parse_u64(get, "REVERIE_ENRICHMENT_FETCH_BUDGET_SECS", 15)?;
        let http_timeout_secs = parse_u64(get, "REVERIE_ENRICHMENT_HTTP_TIMEOUT_SECS", 10)?;
        let max_attempts = parse_u32(get, "REVERIE_ENRICHMENT_MAX_ATTEMPTS", 10)?;
        let cache_ttl_hit_days = parse_u32(get, "REVERIE_ENRICHMENT_CACHE_TTL_HIT_DAYS", 30)?;
        let cache_ttl_miss_days = parse_u32(get, "REVERIE_ENRICHMENT_CACHE_TTL_MISS_DAYS", 7)?;
        let cache_ttl_error_mins = parse_u32(get, "REVERIE_ENRICHMENT_CACHE_TTL_ERROR_MINS", 15)?;

        Ok(Self {
            enabled,
            concurrency,
            poll_idle_secs,
            fetch_budget_secs,
            http_timeout_secs,
            max_attempts,
            cache_ttl_hit_days,
            cache_ttl_miss_days,
            cache_ttl_error_mins,
        })
    }
}

impl WritebackConfig {
    fn from_source(get: &EnvGet<'_>) -> Result<Self, ConfigError> {
        let enabled = parse_bool(get, "REVERIE_WRITEBACK_ENABLED", true)?;
        let concurrency = parse_u32(get, "REVERIE_WRITEBACK_CONCURRENCY", 2)?;
        if !(1..=10).contains(&concurrency) {
            return Err(ConfigError::Invalid {
                var: "REVERIE_WRITEBACK_CONCURRENCY".into(),
                reason: format!("must be 1-10, got {concurrency}"),
            });
        }
        let poll_idle_secs = parse_u64(get, "REVERIE_WRITEBACK_POLL_IDLE_SECS", 5)?;
        let max_attempts = parse_u32(get, "REVERIE_WRITEBACK_MAX_ATTEMPTS", 10)?;
        Ok(Self {
            enabled,
            concurrency,
            poll_idle_secs,
            max_attempts,
        })
    }
}

impl CoverConfig {
    fn from_source(get: &EnvGet<'_>) -> Result<Self, ConfigError> {
        let max_bytes = parse_u64(get, "REVERIE_COVER_MAX_BYTES", 10_485_760)?;
        let download_timeout_secs = parse_u64(get, "REVERIE_COVER_DOWNLOAD_TIMEOUT_SECS", 30)?;
        let min_long_edge_px = parse_u32(get, "REVERIE_COVER_MIN_LONG_EDGE_PX", 1000)?;
        let redirect_limit = parse_u32(get, "REVERIE_COVER_REDIRECT_LIMIT", 3)? as usize;

        Ok(Self {
            max_bytes,
            download_timeout_secs,
            min_long_edge_px,
            redirect_limit,
        })
    }
}

impl OpdsConfig {
    fn from_source(get: &EnvGet<'_>) -> Result<Self, ConfigError> {
        let enabled = parse_bool(get, "REVERIE_OPDS_ENABLED", true)?;
        let page_size = parse_u32(get, "REVERIE_OPDS_PAGE_SIZE", 50)?;
        if !(1..=500).contains(&page_size) {
            return Err(ConfigError::Invalid {
                var: "REVERIE_OPDS_PAGE_SIZE".into(),
                reason: format!("must be 1-500, got {page_size}"),
            });
        }
        let realm = get("REVERIE_OPDS_REALM").unwrap_or_else(|| "Reverie OPDS".into());
        if realm.contains('"') {
            return Err(ConfigError::Invalid {
                var: "REVERIE_OPDS_REALM".into(),
                reason: "must not contain '\"'".into(),
            });
        }
        let public_url = match get("REVERIE_PUBLIC_URL").filter(|s| !s.is_empty()) {
            Some(s) => Some(url::Url::parse(&s).map_err(|e| ConfigError::Invalid {
                var: "REVERIE_PUBLIC_URL".into(),
                reason: e.to_string(),
            })?),
            None => None,
        };
        if enabled && public_url.is_none() {
            return Err(ConfigError::Invalid {
                var: "REVERIE_PUBLIC_URL".into(),
                reason: "required when REVERIE_OPDS_ENABLED=true".into(),
            });
        }
        Ok(Self {
            enabled,
            page_size,
            realm,
            public_url,
        })
    }
}

impl SecurityConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_source(&process_env_get)
    }

    fn from_source(get: &EnvGet<'_>) -> Result<Self, ConfigError> {
        let behind_https = parse_bool(get, "REVERIE_BEHIND_HTTPS", false)?;
        let hsts_include_subdomains = parse_bool(get, "REVERIE_HSTS_INCLUDE_SUBDOMAINS", false)?;
        let hsts_preload = parse_bool(get, "REVERIE_HSTS_PRELOAD", false)?;

        if hsts_include_subdomains && !behind_https {
            return Err(ConfigError::Invalid {
                var: "REVERIE_HSTS_INCLUDE_SUBDOMAINS".into(),
                reason: "requires REVERIE_BEHIND_HTTPS=true".into(),
            });
        }
        if hsts_preload && !hsts_include_subdomains {
            return Err(ConfigError::Invalid {
                var: "REVERIE_HSTS_PRELOAD".into(),
                reason: "requires REVERIE_HSTS_INCLUDE_SUBDOMAINS=true".into(),
            });
        }

        let csp_report_endpoint = match get("REVERIE_CSP_REPORT_ENDPOINT").filter(|s| !s.is_empty())
        {
            Some(s) => {
                // Header-injection guard: this URL flows into a response header
                // (Reporting-Endpoints / report-to / report-uri). Reject quote
                // and CR/LF/semicolon which would split or escape values.
                if s.chars().any(|c| matches!(c, '"' | ';' | '\r' | '\n')) {
                    return Err(ConfigError::Invalid {
                        var: "REVERIE_CSP_REPORT_ENDPOINT".into(),
                        reason: "must not contain \" ; CR or LF".into(),
                    });
                }
                let parsed = url::Url::parse(&s).map_err(|e| ConfigError::Invalid {
                    var: "REVERIE_CSP_REPORT_ENDPOINT".into(),
                    reason: e.to_string(),
                })?;
                if !matches!(parsed.scheme(), "http" | "https") {
                    return Err(ConfigError::Invalid {
                        var: "REVERIE_CSP_REPORT_ENDPOINT".into(),
                        reason: format!("scheme must be http or https, got '{}'", parsed.scheme()),
                    });
                }
                Some(parsed)
            }
            None => None,
        };

        let frontend_dist_path = get("REVERIE_FRONTEND_DIST_PATH")
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from);

        Ok(Self {
            behind_https,
            hsts_include_subdomains,
            hsts_preload,
            csp_report_endpoint,
            frontend_dist_path,
            csp_html_header: None,
            csp_api_header: None,
        })
    }

    /// HSTS response-header value. `None` when `behind_https=false` — the
    /// middleware must not emit HSTS on plaintext HTTP or the browser would
    /// refuse to talk to the host on its next TLS-less request. The composed
    /// string is static ASCII; `from_str` panics on the impossible case so
    /// any future composition bug surfaces loudly instead of silently
    /// dropping the header.
    pub fn hsts_header_value(&self) -> Option<axum::http::HeaderValue> {
        if !self.behind_https {
            return None;
        }
        let mut v = String::from("max-age=31536000");
        if self.hsts_include_subdomains {
            v.push_str("; includeSubDomains");
        }
        if self.hsts_preload {
            v.push_str("; preload");
        }
        Some(axum::http::HeaderValue::from_str(&v).unwrap_or_else(|e| {
            panic!("HSTS string is not a valid HTTP header value ({e}): {v:?}")
        }))
    }

    /// `Reporting-Endpoints: csp-endpoint="<url>"`. `None` when
    /// `csp_report_endpoint` is unset. The URL was validated by
    /// [`Self::from_env`] (no `"` `;` CR or LF; valid `url::Url`); `as_str()`
    /// returns the canonical percent-encoded form. `from_str` panics on the
    /// impossible case rather than silently dropping the header.
    pub fn reporting_endpoints_header_value(&self) -> Option<axum::http::HeaderValue> {
        let url = self.csp_report_endpoint.as_ref()?;
        let v = format!("csp-endpoint=\"{}\"", url.as_str());
        Some(axum::http::HeaderValue::from_str(&v).unwrap_or_else(|e| {
            panic!("Reporting-Endpoints value is not a valid HTTP header value ({e}): {v:?}")
        }))
    }
}

fn parse_bool(get: &EnvGet<'_>, var: &str, default: bool) -> Result<bool, ConfigError> {
    // Strict: accept only lowercase "true"/"false" (exact match). The previous
    // lenient form accepted "1"/"0"/"yes"/"no" with case-insensitivity; it was
    // tightened in UNK-106 so operator-facing values have a single canonical
    // form. Pre-MVP: no operators to migrate.
    get(var).map_or(Ok(default), |v| match v.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ConfigError::Invalid {
            var: var.into(),
            reason: format!("expected 'true' or 'false', got '{v}'"),
        }),
    })
}

fn parse_u32(get: &EnvGet<'_>, var: &str, default: u32) -> Result<u32, ConfigError> {
    get(var).map_or(Ok(default), |v| {
        v.parse::<u32>().map_err(|e| ConfigError::Invalid {
            var: var.into(),
            reason: e.to_string(),
        })
    })
}

fn parse_u64(get: &EnvGet<'_>, var: &str, default: u64) -> Result<u64, ConfigError> {
    get(var).map_or(Ok(default), |v| {
        v.parse::<u64>().map_err(|e| ConfigError::Invalid {
            var: var.into(),
            reason: e.to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Build an `EnvGet` closure backed by an in-memory map. Tests inject
    /// env via this rather than mutating process env (UNK-100 — eliminates
    /// the `sqlx::test` race that `ENV_LOCK` + `unsafe { env::set_var }` was
    /// working around).
    fn env_for(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    const BASE_VARS: &[(&str, &str)] = &[
        ("DATABASE_URL", "postgres://test@localhost/reverie_dev"),
        ("OIDC_ISSUER_URL", "https://auth.example.com"),
        ("OIDC_CLIENT_ID", "test"),
        ("OIDC_CLIENT_SECRET", "secret"),
        ("OIDC_REDIRECT_URI", "http://localhost:3000/auth/callback"),
        // OPDS: default enabled=true requires PUBLIC_URL. Tests that don't
        // care about OPDS disable it here.
        ("REVERIE_OPDS_ENABLED", "false"),
    ];

    fn with_overrides(extra: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = BASE_VARS
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        for (k, v) in extra {
            if let Some(slot) = out.iter_mut().find(|(kk, _)| kk == k) {
                slot.1 = (*v).to_string();
            } else {
                out.push(((*k).to_string(), (*v).to_string()));
            }
        }
        out
    }

    fn without_keys(keys: &[&str]) -> Vec<(String, String)> {
        BASE_VARS
            .iter()
            .filter(|(k, _)| !keys.contains(k))
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// `env_for` variant taking owned-string slices — for callers that build
    /// the var list via `with_overrides` / `without_keys`.
    fn env_for_owned(vars: &[(String, String)]) -> impl Fn(&str) -> Option<String> + use<'_> {
        let map: HashMap<&str, &str> = vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        move |k| map.get(k).map(|s| (*s).to_string())
    }

    #[test]
    fn from_env_with_defaults() {
        let config = Config::from_source(&env_for(BASE_VARS)).unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.database_url, "postgres://test@localhost/reverie_dev");
        assert_eq!(config.library_path, "./library");
        assert_eq!(config.ingestion_path, "./ingestion");
        assert_eq!(config.quarantine_path, "./quarantine");
        // Falls back to DATABASE_URL when DATABASE_URL_INGESTION is unset
        assert_eq!(
            config.ingestion_database_url,
            "postgres://test@localhost/reverie_dev"
        );
        assert_eq!(
            config.format_priority,
            vec![
                ManifestationFormat::Epub,
                ManifestationFormat::Pdf,
                ManifestationFormat::Mobi,
                ManifestationFormat::Azw3,
                ManifestationFormat::Cbz,
                ManifestationFormat::Cbr,
            ]
        );
        assert_eq!(config.cleanup_mode, CleanupMode::All);
        // Enrichment defaults
        assert!(config.enrichment.enabled);
        assert_eq!(config.enrichment.concurrency, 2);
        assert_eq!(config.enrichment.max_attempts, 10);
        assert_eq!(config.cover.max_bytes, 10_485_760);
        assert_eq!(config.cover.min_long_edge_px, 1000);
        assert_eq!(config.cover.redirect_limit, 3);
        // Writeback defaults
        assert!(config.writeback.enabled);
        assert_eq!(config.writeback.concurrency, 2);
        assert_eq!(config.writeback.poll_idle_secs, 5);
        assert_eq!(config.writeback.max_attempts, 10);
        assert_eq!(config.openlibrary_base_url, "https://openlibrary.org");
        assert!(config.googlebooks_api_key.is_none());
        assert!(config.hardcover_api_token.is_none());
        assert!(config.operator_contact.is_none());
    }

    #[test]
    fn user_agent_without_contact_reports_unidentified() {
        let config = Config::from_source(&env_for(BASE_VARS)).unwrap();
        let ua = config.user_agent();
        assert!(ua.starts_with("Reverie/"), "missing Reverie/ prefix: {ua}");
        assert!(ua.ends_with("(unidentified)"), "unexpected suffix: {ua}");
    }

    #[test]
    fn user_agent_with_contact_embeds_identifier() {
        let vars = with_overrides(&[("REVERIE_OPERATOR_CONTACT", "ops@example.com")]);
        let config = Config::from_source(&env_for_owned(&vars)).unwrap();
        assert_eq!(config.operator_contact.as_deref(), Some("ops@example.com"));
        let ua = config.user_agent();
        assert!(ua.contains("(ops@example.com)"), "missing contact: {ua}");
        assert!(ua.starts_with("Reverie/"), "missing Reverie/ prefix: {ua}");
    }

    #[test]
    fn from_env_rejects_concurrency_out_of_range() {
        let vars = with_overrides(&[("REVERIE_ENRICHMENT_CONCURRENCY", "11")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        assert!(err.to_string().contains("REVERIE_ENRICHMENT_CONCURRENCY"));
    }

    #[test]
    fn from_env_all_vars() {
        let vars = with_overrides(&[
            ("DATABASE_URL", "postgres://custom@localhost/reverie_dev"),
            ("REVERIE_PORT", "8080"),
            ("REVERIE_LIBRARY_PATH", "/data/library"),
            ("REVERIE_INGESTION_PATH", "/data/ingestion"),
            ("REVERIE_QUARANTINE_PATH", "/data/quarantine"),
            ("RUST_LOG", "debug"),
        ]);
        let config = Config::from_source(&env_for_owned(&vars)).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(
            config.database_url,
            "postgres://custom@localhost/reverie_dev"
        );
        assert_eq!(config.library_path, "/data/library");
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn from_env_missing_database_url() {
        let vars = without_keys(&["DATABASE_URL"]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        assert!(err.to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn from_env_custom_ingestion_url_and_format_priority() {
        let vars = with_overrides(&[
            (
                "DATABASE_URL_INGESTION",
                "postgres://ingestion@localhost/reverie_dev",
            ),
            ("REVERIE_FORMAT_PRIORITY", "pdf, EPUB , mobi"),
        ]);
        let config = Config::from_source(&env_for_owned(&vars)).unwrap();
        assert_eq!(
            config.ingestion_database_url,
            "postgres://ingestion@localhost/reverie_dev"
        );
        assert_eq!(
            config.format_priority,
            vec![
                ManifestationFormat::Pdf,
                ManifestationFormat::Epub,
                ManifestationFormat::Mobi,
            ]
        );
    }

    #[test]
    fn from_env_rejects_unsupported_format_priority() {
        let vars = with_overrides(&[("REVERIE_FORMAT_PRIORITY", "epub,djvu")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("djvu"), "expected djvu in error: {msg}");
        assert!(
            msg.contains("REVERIE_FORMAT_PRIORITY"),
            "expected var name in error: {msg}"
        );
    }

    #[test]
    fn opds_enabled_without_public_url_errors() {
        let vars = with_overrides(&[("REVERIE_OPDS_ENABLED", "true")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("REVERIE_PUBLIC_URL"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn opds_page_size_out_of_range_errors() {
        for bad in ["0", "501"] {
            let vars = with_overrides(&[("REVERIE_OPDS_PAGE_SIZE", bad)]);
            let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("REVERIE_OPDS_PAGE_SIZE"),
                "page_size={bad} did not surface var name: {msg}"
            );
        }
    }

    #[test]
    fn opds_realm_with_double_quote_errors() {
        let vars = with_overrides(&[("REVERIE_OPDS_REALM", "bad\"quote")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("REVERIE_OPDS_REALM"),
            "expected realm error: {msg}"
        );
    }

    #[test]
    fn opds_enabled_with_valid_public_url_parses() {
        let vars = with_overrides(&[
            ("REVERIE_OPDS_ENABLED", "true"),
            ("REVERIE_PUBLIC_URL", "https://reverie.example.com/"),
        ]);
        let config = Config::from_source(&env_for_owned(&vars)).unwrap();
        assert!(config.opds.enabled);
        assert_eq!(
            config.opds.public_url.as_ref().map(url::Url::as_str),
            Some("https://reverie.example.com/")
        );
    }

    #[test]
    fn security_defaults_all_off() {
        let cfg = SecurityConfig::from_source(&env_for(&[])).unwrap();
        assert!(!cfg.behind_https);
        assert!(!cfg.hsts_include_subdomains);
        assert!(!cfg.hsts_preload);
        assert!(cfg.csp_report_endpoint.is_none());
        assert!(cfg.frontend_dist_path.is_none());
    }

    #[test]
    fn security_hsts_subdomains_without_https_errors() {
        let err =
            SecurityConfig::from_source(&env_for(&[("REVERIE_HSTS_INCLUDE_SUBDOMAINS", "true")]))
                .unwrap_err();
        assert!(
            err.to_string().contains("REVERIE_HSTS_INCLUDE_SUBDOMAINS"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn security_hsts_preload_without_subdomains_errors() {
        let err = SecurityConfig::from_source(&env_for(&[
            ("REVERIE_BEHIND_HTTPS", "true"),
            ("REVERIE_HSTS_PRELOAD", "true"),
        ]))
        .unwrap_err();
        assert!(
            err.to_string().contains("REVERIE_HSTS_PRELOAD"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn security_hsts_full_stack_ok() {
        let cfg = SecurityConfig::from_source(&env_for(&[
            ("REVERIE_BEHIND_HTTPS", "true"),
            ("REVERIE_HSTS_INCLUDE_SUBDOMAINS", "true"),
            ("REVERIE_HSTS_PRELOAD", "true"),
        ]))
        .unwrap();
        assert!(cfg.behind_https);
        assert!(cfg.hsts_include_subdomains);
        assert!(cfg.hsts_preload);
        let v = cfg.hsts_header_value().unwrap();
        assert_eq!(
            v.to_str().unwrap(),
            "max-age=31536000; includeSubDomains; preload"
        );
    }

    #[test]
    fn security_hsts_header_absent_when_plaintext() {
        let cfg = SecurityConfig::from_source(&env_for(&[])).unwrap();
        assert!(cfg.hsts_header_value().is_none());
    }

    #[test]
    fn security_report_endpoint_bad_scheme_errors() {
        let err = SecurityConfig::from_source(&env_for(&[(
            "REVERIE_CSP_REPORT_ENDPOINT",
            "ftp://bad.example",
        )]))
        .unwrap_err();
        assert!(err.to_string().contains("scheme"), "unexpected: {err}");
    }

    #[test]
    fn security_report_endpoint_malformed_url_errors() {
        let err =
            SecurityConfig::from_source(&env_for(&[("REVERIE_CSP_REPORT_ENDPOINT", "not a url")]))
                .unwrap_err();
        assert!(
            err.to_string().contains("REVERIE_CSP_REPORT_ENDPOINT"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn security_report_endpoint_injection_chars_errors() {
        for bad in [
            "https://ok.example/\";x=y",
            "https://ok.example/;evil",
            "https://ok.example/\r\nX-Injected: 1",
        ] {
            let err =
                SecurityConfig::from_source(&env_for(&[("REVERIE_CSP_REPORT_ENDPOINT", bad)]))
                    .unwrap_err();
            assert!(
                err.to_string().contains("must not contain"),
                "unexpected: {err}"
            );
        }
    }

    #[test]
    fn security_report_endpoint_happy_path() {
        let cfg = SecurityConfig::from_source(&env_for(&[(
            "REVERIE_CSP_REPORT_ENDPOINT",
            "https://log.example/csp",
        )]))
        .unwrap();
        let url = cfg.csp_report_endpoint.as_ref().unwrap();
        assert_eq!(url.as_str(), "https://log.example/csp");
        let hv = cfg.reporting_endpoints_header_value().unwrap();
        assert_eq!(
            hv.to_str().unwrap(),
            r#"csp-endpoint="https://log.example/csp""#
        );
    }

    #[test]
    fn security_parse_bool_rejects_legacy_truthy() {
        // UNK-110: strict form rejects the old "1"/"yes" spellings.
        let err =
            SecurityConfig::from_source(&env_for(&[("REVERIE_BEHIND_HTTPS", "yes")])).unwrap_err();
        assert!(err.to_string().contains("REVERIE_BEHIND_HTTPS"));
    }

    #[test]
    fn from_env_invalid_port() {
        let vars = with_overrides(&[("REVERIE_PORT", "not_a_number")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        assert!(err.to_string().contains("REVERIE_PORT"));
    }

    #[test]
    fn from_env_invalid_cleanup_mode() {
        let vars = with_overrides(&[("REVERIE_CLEANUP_MODE", "archive")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        assert!(
            err.to_string().contains("REVERIE_CLEANUP_MODE"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn opds_page_size_boundary_values_accepted() {
        for boundary in ["1", "500"] {
            let vars = with_overrides(&[("REVERIE_OPDS_PAGE_SIZE", boundary)]);
            let cfg = Config::from_source(&env_for_owned(&vars))
                .unwrap_or_else(|e| panic!("page_size={boundary} should be accepted: {e}"));
            assert_eq!(cfg.opds.page_size, boundary.parse::<u32>().unwrap());
        }
    }

    #[test]
    fn from_env_rejects_zero_enrichment_concurrency() {
        let vars = with_overrides(&[("REVERIE_ENRICHMENT_CONCURRENCY", "0")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        assert!(err.to_string().contains("REVERIE_ENRICHMENT_CONCURRENCY"));
    }

    #[test]
    fn from_env_rejects_zero_writeback_concurrency() {
        let vars = with_overrides(&[("REVERIE_WRITEBACK_CONCURRENCY", "0")]);
        let err = Config::from_source(&env_for_owned(&vars)).unwrap_err();
        assert!(err.to_string().contains("REVERIE_WRITEBACK_CONCURRENCY"));
    }

    // Cover the production wiring `&process_env_get`. CARGO_PKG_NAME is set by
    // cargo for every test run; UNSET_REVERIE_TEST_VAR is reserved nowhere.
    #[test]
    fn process_env_get_reads_process_env_for_set_var() {
        let v = super::process_env_get("CARGO_PKG_NAME");
        assert_eq!(v.as_deref(), Some("reverie-api"));
    }

    #[test]
    fn process_env_get_returns_none_for_unset_var() {
        assert!(super::process_env_get("UNSET_REVERIE_TEST_VAR").is_none());
    }
}
