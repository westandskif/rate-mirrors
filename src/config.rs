use crate::mirror::Mirror;
use crate::target_configs::arch4edu::Arch4eduTarget;
use crate::target_configs::archarm::ArcharmTarget;
use crate::target_configs::archlinux::ArchTarget;
use crate::target_configs::archlinuxcn::ArchCNTarget;
use crate::target_configs::arcolinux::ArcoLinuxTarget;
use crate::target_configs::artix::ArtixTarget;
use crate::target_configs::blackarch::BlackArchTarget;
use crate::target_configs::cachyos::CachyOSTarget;
use crate::target_configs::chaotic::ChaoticTarget;
// use crate::target_configs::debian::DebianTarget;
use crate::target_configs::endeavouros::EndeavourOSTarget;
use crate::target_configs::manjaro::ManjaroTarget;
use crate::target_configs::openbsd::OpenBSDTarget;
use crate::target_configs::rebornos::RebornOSTarget;
use crate::target_configs::stdin::StdinTarget;
// use crate::target_configs::ubuntu::UbuntuTarget;
use ambassador::{Delegate, delegatable_trait};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Duration;
use thiserror::Error;
use tokio::runtime::Runtime;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub enum Protocol {
    Http,
    Https,
}

impl FromStr for Protocol {
    type Err = &'static str;
    fn from_str(protocol: &str) -> Result<Self, Self::Err> {
        match protocol {
            "http" => Ok(Protocol::Http),
            "https" => Ok(Protocol::Https),
            _ => Err("could not parse protocol"),
        }
    }
}

#[derive(Error)]
pub enum AppError {
    #[error("do not run rate-mirrors with root permissions")]
    Root,
    #[error("failed to connect to {0}, consider increasing fetch-mirrors-timeout")]
    RequestTimeout(String),
    #[error("{0}")]
    RequestError(String),
    #[error("HTTP {status} from {url}")]
    HttpError { status: u16, url: String },
    #[error("no mirrors after filtering")]
    NoMirrorsAfterFiltering,
    #[error("all speed tests failed")]
    SpeedTestsFailed,
    #[error("no mirror output produced")]
    BlankOutput,
    #[error("stdout closed")]
    StdoutBrokenPipe,
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> AppError {
        if err.is_timeout() {
            AppError::RequestTimeout(err.url().map(|u| u.to_string()).unwrap_or_default())
        } else {
            AppError::RequestError(err.to_string())
        }
    }
}

#[delegatable_trait]
pub trait LogFormatter {
    fn format_comment(&self, message: impl fmt::Display) -> String;
    fn format_mirror(&self, mirror: &Mirror) -> String;
}

#[delegatable_trait]
pub trait FetchMirrors {
    fn fetch_mirrors(&self, tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError>;
}

#[derive(Debug, Subcommand, Clone, Delegate)]
#[delegate(FetchMirrors)]
#[delegate(LogFormatter)]
pub enum Target {
    /// accepts lines of urls OR lines with tab-separated urls and countries
    Stdin(StdinTarget),

    /// test archlinux mirrors
    Arch(ArchTarget),

    /// test arch4edu mirrors
    #[command(name = "arch4edu")]
    Arch4edu(Arch4eduTarget),

    /// test archlinuxcn mirrors
    #[command(name = "archlinuxcn")]
    ArchCN(ArchCNTarget),

    /// test archlinuxarm mirrors
    Archarm(ArcharmTarget),

    /// test artix mirrors
    Artix(ArtixTarget),

    /// test arcolinux mirrors
    #[command(name = "arcolinux")]
    ArcoLinux(ArcoLinuxTarget),

    /// test blackarch mirrors
    #[command(name = "blackarch")]
    BlackArch(BlackArchTarget),

    /// test cachyos mirrors
    #[command(name = "cachyos")]
    CachyOS(CachyOSTarget),

    /// test chaotic-aur mirrors
    #[command(name = "chaotic-aur")]
    Chaotic(ChaoticTarget),

    /// test endeavouros mirrors
    #[command(name = "endeavouros")]
    EndeavourOS(EndeavourOSTarget),

    /// test manjaro mirrors
    Manjaro(ManjaroTarget),

    /// test OpenBSD mirrors
    #[command(name = "openbsd")]
    OpenBSD(OpenBSDTarget),

    /// test rebornos mirrors
    #[command(name = "rebornos")]
    RebornOS(RebornOSTarget),
}

fn parse_positive_usize(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|e| format!("{e}"))?;
    if n == 0 {
        return Err("value must be at least 1".into());
    }
    Ok(n)
}

#[derive(Debug, Parser)]
#[command(
    name = "rate-mirrors config",
    about,
    version,
    rename_all = "kebab-case",
    rename_all_env = "SCREAMING_SNAKE_CASE"
)]
pub struct Config {
    /// Per-mirror speed test timeout in milliseconds
    #[command(subcommand)]
    pub target: Target,

    /// Test only specified protocols (can be passed multiple times)
    #[arg(env = "RATE_MIRRORS_PROTOCOL", long = "protocol", name = "protocol")]
    pub protocols: Vec<Protocol>,

    /// Per-mirror speed test timeout in milliseconds
    #[arg(env = "RATE_MIRRORS_PER_MIRROR_TIMEOUT", long, default_value = "8000")]
    pub per_mirror_timeout: u64,

    /// Minimum downloading time, required to measure mirror speed,
    /// in milliseconds
    #[arg(env = "RATE_MIRRORS_MIN_PER_MIRROR", long, default_value = "300")]
    pub min_per_mirror: u64,

    /// Maximum downloading time, required to measure mirror speed,
    /// in milliseconds
    #[arg(env = "RATE_MIRRORS_MAX_PER_MIRROR", long, default_value = "1000")]
    pub max_per_mirror: u64,

    /// Minimum number of bytes to be downloaded,
    /// required to measure mirror speed
    #[arg(
        env = "RATE_MIRRORS_MIN_BYTES_PER_MIRROR",
        long,
        default_value = "70000"
    )]
    pub min_bytes_per_mirror: usize,
    /// Relative speed-jitter threshold used for early stopping.
    ///
    /// After the minimum measurement requirements are met, stop when the
    /// standard deviation of the last `--eps-checks` chunk speeds divided
    /// by their mean is at most this value. Smaller values require steadier
    /// recent throughput and may make tests run longer.
    ///
    /// For example, 0.0625 requires the standard deviation to be at most
    /// 6.25% of the mean.
    ///
    /// This is a stability heuristic, not a confidence or error bound.
    #[arg(
        env = "RATE_MIRRORS_EPS",
        long,
        default_value = "0.0625",
        verbatim_doc_comment
    )]
    pub eps: f64,

    /// Per-mirror: after min measurement time elapsed, check such number of
    /// subsequently downloaded data chunks whether speed variations are less
    /// than "eps"
    #[arg(env = "RATE_MIRRORS_EPS_CHECKS", long, default_value = "40")]
    pub eps_checks: usize,

    /// Number of simultaneous speed tests
    #[arg(env = "RATE_MIRRORS_CONCURRENCY", long, default_value = "16")]
    pub concurrency: usize,

    /// Number of simultaneous speed tests for mirrors with unknown country
    #[arg(
        env = "RATE_MIRRORS_CONCURRENCY_FOR_UNLABELED",
        long,
        default_value = "40"
    )]
    pub concurrency_for_unlabeled: usize,

    /// Max number of jumps between countries, when finding top mirrors
    #[arg(env = "RATE_MIRRORS_MAX_JUMPS", long, default_value = "7")]
    pub max_jumps: usize,

    /// Entry country - first country (+ its neighbours) to test.
    /// You don't need to change it unless you are just curious.
    #[arg(
        env = "RATE_MIRRORS_ENTRY_COUNTRY",
        long,
        default_value = "US",
        verbatim_doc_comment
    )]
    pub entry_country: String,

    /// Exclude countries from mirror selection (comma-separated 2-letter ISO country codes).
    /// Use ZZ to filter out mirrors with undefined country.
    /// Mutually exclusive with --include-countries.
    #[arg(
        env = "RATE_MIRRORS_EXCLUDE_COUNTRIES",
        long,
        value_name = "country-codes",
        verbatim_doc_comment
    )]
    pub exclude_countries: Option<String>,

    /// Include countries in mirror selection (comma-separated 2-letter ISO country codes).
    /// Use ZZ to include mirrors with undefined country.
    /// Mutually exclusive with --exclude-countries.
    #[arg(
        conflicts_with = "exclude_countries",
        env = "RATE_MIRRORS_INCLUDE_COUNTRIES",
        long,
        value_name = "country-codes",
        verbatim_doc_comment
    )]
    pub include_countries: Option<String>,

    /// Neighbor country to test per country
    #[arg(
        env = "RATE_MIRRORS_COUNTRY_NEIGHBORS_PER_COUNTRY",
        long,
        default_value = "3"
    )]
    pub country_neighbors_per_country: usize,

    /// Number of mirrors to test per country
    #[arg(
        env = "RATE_MIRRORS_COUNTRY_TEST_MIRRORS_PER_COUNTRY",
        long,
        default_value = "2"
    )]
    pub country_test_mirrors_per_country: usize,

    /// Number of top mirrors to retest
    #[arg(
        env = "RATE_MIRRORS_TOP_MIRRORS_NUMBER_TO_RETEST",
        long,
        default_value = "5"
    )]
    pub top_mirrors_number_to_retest: usize,

    /// Max number of mirrors to output
    #[arg(env = "RATE_MIRRORS_MAX_MIRRORS_TO_OUTPUT", long, value_parser = parse_positive_usize)]
    pub max_mirrors_to_output: Option<usize>,

    /// Filename to save the output to in case of success
    #[arg(env = "RATE_MIRRORS_SAVE", long = "save", verbatim_doc_comment)]
    pub save_to_file: Option<String>,

    /// Allow running by root
    #[arg(env = "RATE_MIRRORS_ALLOW_ROOT", long)]
    pub allow_root: bool,

    /// Disable printing comments
    #[arg(env = "RATE_MIRRORS_DISABLE_COMMENTS", long)]
    pub disable_comments: bool,

    /// Disable printing comments to output file
    #[arg(env = "RATE_MIRRORS_DISABLE_COMMENTS_IN_FILE", long)]
    pub disable_comments_in_file: bool,

    /// Exit with error instead of outputting untested mirrors when all speed tests fail
    #[arg(env = "RATE_MIRRORS_DISABLE_UNTESTED_FALLBACK", long)]
    pub disable_untested_fallback: bool,

    /// Pre-parsed set of excluded country codes (lowercase)
    #[arg(skip)]
    pub excluded_countries_set: HashSet<String>,

    /// Pre-parsed set of included country codes (lowercase)
    #[arg(skip)]
    pub included_countries_set: HashSet<String>,
}

impl Config {
    pub fn new() -> Self {
        Self::parse().populate_country_sets()
    }

    fn populate_country_sets(mut self) -> Self {
        fn parse_ccs(s: &Option<String>) -> HashSet<String> {
            s.as_ref()
                .map(|s| {
                    s.split(',')
                        .map(|c| c.trim().to_ascii_lowercase())
                        .filter(|c| !c.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        }

        self.excluded_countries_set = parse_ccs(&self.exclude_countries);
        self.included_countries_set = parse_ccs(&self.include_countries);
        self
    }

    pub fn is_country_excluded(&self, code: &str) -> bool {
        let code = code.to_ascii_lowercase();
        self.excluded_countries_set.contains(&code)
            || (self.include_countries.is_some() && !self.included_countries_set.contains(&code))
    }

    pub fn is_protocol_allowed_for_url(&self, url: &Url) -> bool {
        if self.protocols.is_empty() {
            matches!(url.scheme(), "http" | "https")
        } else {
            url.scheme()
                .parse()
                .map(|p| self.protocols.contains(&p))
                .unwrap_or(false)
        }
    }
}

pub fn default_client_builder() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME").replace('_', "-"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| AppError::RequestError(format!("failed to build HTTP client: {}", e)))
}

fn convert_reqwest_error(e: reqwest::Error, url: &str) -> AppError {
    if e.is_timeout() {
        AppError::RequestTimeout(url.to_string())
    } else {
        AppError::RequestError(format!("failed to connect to {}: {}", url, e))
    }
}

fn is_retryable_request_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_body()
}

fn exponential_backoff(attempt: usize) -> Duration {
    Duration::from_millis(250_u64.saturating_mul(1_u64 << attempt.min(20)))
}

fn retry_delay(headers: &reqwest::header::HeaderMap, attempt: usize) -> Duration {
    const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

    let retry_after = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let value = value.trim();
            if let Ok(seconds) = value.parse::<u64>() {
                return Some(Duration::from_secs(seconds));
            }

            chrono::DateTime::parse_from_rfc2822(value)
                .ok()
                .map(|date| {
                    let remaining = date.with_timezone(&chrono::Utc) - chrono::Utc::now();
                    Duration::from_millis(remaining.num_milliseconds().max(0) as u64)
                })
        });

    retry_after
        .unwrap_or_else(|| exponential_backoff(attempt))
        .min(MAX_RETRY_AFTER)
}

async fn request_with_retry<T, F, Fut, M>(
    client: &reqwest::Client,
    url: &str,
    timeout_ms: u64,
    consume_response: F,
    map_response_error: M,
) -> Result<T, AppError>
where
    F: Fn(reqwest::Response) -> Fut,
    Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
    M: Fn(reqwest::Error, &str) -> AppError,
{
    const MAX_ATTEMPTS: usize = 3;

    for attempt in 0..MAX_ATTEMPTS {
        let response = match client
            .get(url)
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) if is_retryable_request_error(&error) && attempt + 1 < MAX_ATTEMPTS => {
                tokio::time::sleep(exponential_backoff(attempt)).await;
                continue;
            }
            Err(error) => return Err(convert_reqwest_error(error, url)),
        };

        let status = response.status();
        let retryable_status = status == reqwest::StatusCode::REQUEST_TIMEOUT
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || status.is_server_error();

        if retryable_status && attempt + 1 < MAX_ATTEMPTS {
            let delay = retry_delay(response.headers(), attempt);
            drop(response);
            tokio::time::sleep(delay).await;
            continue;
        }

        if !status.is_success() {
            return Err(AppError::HttpError {
                status: status.as_u16(),
                url: url.to_string(),
            });
        }

        match consume_response(response).await {
            Ok(value) => return Ok(value),
            // The response consumer only reads the body. Retry transfer/read errors;
            // JSON syntax parsing is handled after this loop.
            Err(_error) if attempt + 1 < MAX_ATTEMPTS => {
                tokio::time::sleep(exponential_backoff(attempt)).await;
            }
            Err(error) => return Err(map_response_error(error, url)),
        }
    }

    unreachable!("request loop always returns or retries")
}

pub fn fetch_json<T: DeserializeOwned>(url: &str, timeout_ms: u64) -> Result<T, AppError> {
    let runtime = Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let client = default_client_builder()?;
        let body = request_with_retry(
            &client,
            url,
            timeout_ms,
            |response| async move { response.bytes().await },
            |error, url| {
                if error.is_timeout() || error.is_connect() {
                    convert_reqwest_error(error, url)
                } else {
                    AppError::RequestError(format!(
                        "failed to read response from {}: {}",
                        url, error
                    ))
                }
            },
        )
        .await?;

        // Parse JSON only after body transfer succeeds, so invalid JSON isn't retried.
        serde_json::from_slice::<T>(body.as_ref()).map_err(|error| {
            AppError::RequestError(format!("failed to decode JSON from {}: {}", url, error))
        })
    });
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

pub fn fetch_text(url: &str, timeout_ms: u64) -> Result<String, AppError> {
    let runtime = Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let client = default_client_builder()?;
        request_with_retry(
            &client,
            url,
            timeout_ms,
            |response| async move { response.text_with_charset("utf-8").await },
            |error, url| {
                if error.is_timeout() || error.is_connect() {
                    convert_reqwest_error(error, url)
                } else {
                    AppError::RequestError(format!(
                        "failed to read response from {}: {}",
                        url, error
                    ))
                }
            },
        )
        .await
    });
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

/// Fetches text content from either a remote URL or a local file path.
/// URLs are detected via `Url::parse`; everything else is treated as a filesystem path.
/// Used by mirror-list and mirror-source options that can point at a URL or a file.
pub fn fetch_text_or_file(path_or_url: &str, timeout_ms: u64) -> Result<String, AppError> {
    if Url::parse(path_or_url).is_ok() {
        fetch_text(path_or_url, timeout_ms)
    } else {
        fs::read_to_string(path_or_url)
            .map_err(|e| AppError::RequestError(format!("failed to read mirror source: {}", e)))
    }
}

/// Same as `fetch_text_or_file` but deserializes the result as JSON.
/// Supports both remote status endpoints and local JSON files for the mirror list source.
pub fn fetch_json_or_file<T: DeserializeOwned>(
    path_or_url: &str,
    timeout_ms: u64,
) -> Result<T, AppError> {
    if Url::parse(path_or_url).is_ok() {
        fetch_json(path_or_url, timeout_ms)
    } else {
        let content = fs::read_to_string(path_or_url)
            .map_err(|e| AppError::RequestError(format!("failed to read mirror source: {}", e)))?;
        serde_json::from_str(&content).map_err(|e| {
            AppError::RequestError(format!(
                "failed to decode JSON from mirror source {}: {}",
                path_or_url, e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::archlinux::{ARCH_TIER_1_MIRROR_SOURCE, selected_mirror_source};
    use clap::error::ErrorKind;
    use std::sync::Mutex;

    static MIRROR_SOURCE_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn parse_arch_with_mirror_source_env(
        env_value: Option<&str>,
        args: &[&str],
    ) -> Result<Config, clap::Error> {
        let _guard = MIRROR_SOURCE_ENV_LOCK.lock().unwrap();
        let old_value = std::env::var_os("RATE_MIRRORS_MIRROR_SOURCE");

        unsafe {
            match env_value {
                Some(value) => std::env::set_var("RATE_MIRRORS_MIRROR_SOURCE", value),
                None => std::env::remove_var("RATE_MIRRORS_MIRROR_SOURCE"),
            }
        }

        let result = Config::try_parse_from(args);

        unsafe {
            match old_value {
                Some(value) => std::env::set_var("RATE_MIRRORS_MIRROR_SOURCE", value),
                None => std::env::remove_var("RATE_MIRRORS_MIRROR_SOURCE"),
            }
        }

        result
    }

    fn arch_target(config: &Config) -> &ArchTarget {
        match &config.target {
            Target::Arch(target) => target,
            other => panic!("expected Arch target, got {other:?}"),
        }
    }

    #[test]
    fn arch_fetch_first_tier_only_succeeds_and_selects_tier_1_source() {
        let config = parse_arch_with_mirror_source_env(
            None,
            &["rate-mirrors", "arch", "--fetch-first-tier-only"],
        )
        .unwrap();

        assert_eq!(
            selected_mirror_source(arch_target(&config)),
            ARCH_TIER_1_MIRROR_SOURCE
        );
    }

    #[test]
    fn arch_mirror_source_argument_succeeds() {
        let config = parse_arch_with_mirror_source_env(
            None,
            &[
                "rate-mirrors",
                "arch",
                "--mirror-source",
                "local-status.json",
            ],
        )
        .unwrap();

        assert_eq!(arch_target(&config).mirror_source, "local-status.json");
    }

    #[test]
    fn arch_mirror_source_argument_conflicts_with_fetch_first_tier_only() {
        let err = parse_arch_with_mirror_source_env(
            None,
            &[
                "rate-mirrors",
                "arch",
                "--mirror-source",
                "local-status.json",
                "--fetch-first-tier-only",
            ],
        )
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn arch_mirror_source_env_conflicts_with_fetch_first_tier_only() {
        let err = parse_arch_with_mirror_source_env(
            Some("local-status.json"),
            &["rate-mirrors", "arch", "--fetch-first-tier-only"],
        )
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    fn parse_arch_with_country_filter_env(args: &[&str]) -> Result<Config, clap::Error> {
        let _guard = MIRROR_SOURCE_ENV_LOCK.lock().unwrap();
        let old_exclude = std::env::var_os("RATE_MIRRORS_EXCLUDE_COUNTRIES");
        let old_include = std::env::var_os("RATE_MIRRORS_INCLUDE_COUNTRIES");

        unsafe {
            std::env::remove_var("RATE_MIRRORS_EXCLUDE_COUNTRIES");
            std::env::remove_var("RATE_MIRRORS_INCLUDE_COUNTRIES");
        }

        let result = Config::try_parse_from(args);

        unsafe {
            match old_exclude {
                Some(value) => std::env::set_var("RATE_MIRRORS_EXCLUDE_COUNTRIES", value),
                None => std::env::remove_var("RATE_MIRRORS_EXCLUDE_COUNTRIES"),
            }
            match old_include {
                Some(value) => std::env::set_var("RATE_MIRRORS_INCLUDE_COUNTRIES", value),
                None => std::env::remove_var("RATE_MIRRORS_INCLUDE_COUNTRIES"),
            }
        }

        result
    }

    #[test]
    fn country_filter_modes() {
        let parse = |args: &[&str]| {
            parse_arch_with_country_filter_env(args)
                .unwrap()
                .populate_country_sets()
        };

        struct Case {
            args: &'static [&'static str],
            expected: &'static [(&'static str, bool)],
        }
        let cases = [
            Case {
                args: &["rate-mirrors", "arch"],
                expected: &[("us", false), ("zz", false)],
            },
            Case {
                args: &["rate-mirrors", "--exclude-countries", "US, de,", "arch"],
                expected: &[
                    ("US", true),
                    ("us", true),
                    ("DE", true),
                    ("fr", false),
                    ("zz", false),
                ],
            },
            Case {
                args: &["rate-mirrors", "--exclude-countries", "US,ZZ", "arch"],
                expected: &[("zz", true)],
            },
            Case {
                args: &["rate-mirrors", "--include-countries", "us,ZZ", "arch"],
                expected: &[("us", false), ("zz", false), ("de", true)],
            },
            Case {
                args: &["rate-mirrors", "--include-countries", "us", "arch"],
                expected: &[("zz", true)],
            },
        ];

        for case in cases {
            let config = parse(case.args);
            for (code, excluded) in case.expected {
                assert_eq!(
                    config.is_country_excluded(code),
                    *excluded,
                    "args={:?} code={code}",
                    case.args
                );
            }
        }

        let err = parse_arch_with_country_filter_env(&[
            "rate-mirrors",
            "--exclude-countries",
            "US",
            "--include-countries",
            "DE",
            "arch",
        ])
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }
}

#[cfg(test)]
mod retry_tests {
    use super::{AppError, default_client_builder, request_with_retry};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    struct TestResponse {
        status: u16,
        body: &'static str,
        retry_after: Option<&'static str>,
        content_length: Option<usize>,
    }

    impl TestResponse {
        fn new(status: u16, body: &'static str) -> Self {
            Self {
                status,
                body,
                retry_after: None,
                content_length: None,
            }
        }

        fn with_retry_after(mut self, value: &'static str) -> Self {
            self.retry_after = Some(value);
            self
        }

        fn with_content_length(mut self, value: usize) -> Self {
            self.content_length = Some(value);
            self
        }
    }

    fn start_server(responses: Vec<TestResponse>) -> (String, JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let mut requests = 0;
            for response in responses {
                let deadline = Instant::now() + Duration::from_secs(4);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("timed out waiting for request: {error}"),
                    }
                };

                requests += 1;
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut request = [0_u8; 2048];
                let _ = stream.read(&mut request);

                let reason = match response.status {
                    200 => "OK",
                    404 => "Not Found",
                    408 => "Request Timeout",
                    429 => "Too Many Requests",
                    503 => "Service Unavailable",
                    _ => "Test Response",
                };
                let length = response.content_length.unwrap_or(response.body.len());
                let mut headers = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                    response.status, reason, length
                );
                if let Some(retry_after) = response.retry_after {
                    headers.push_str(&format!("Retry-After: {retry_after}\r\n"));
                }
                headers.push_str("\r\n");
                stream.write_all(headers.as_bytes()).unwrap();
                stream.write_all(response.body.as_bytes()).unwrap();
            }
            requests
        });

        (format!("http://{address}/"), server)
    }

    fn request_body(url: &str) -> Result<Vec<u8>, String> {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = default_client_builder().unwrap();
        runtime
            .block_on(request_with_retry(
                &client,
                url,
                1000,
                |response| async move { response.bytes().await },
                |error, url| {
                    AppError::RequestError(format!("test response failed for {}: {}", url, error))
                },
            ))
            .map(|body| body.to_vec())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn retries_after_service_unavailable() {
        let (url, server) = start_server(vec![
            TestResponse::new(503, "retry"),
            TestResponse::new(200, "ok"),
        ]);
        let result = request_body(&url);
        assert_eq!(server.join().unwrap(), 2);
        assert_eq!(result.unwrap(), b"ok");
    }

    #[test]
    fn retries_after_too_many_requests() {
        let (url, server) = start_server(vec![
            TestResponse::new(429, "retry"),
            TestResponse::new(200, "ok"),
        ]);
        let result = request_body(&url);
        assert_eq!(server.join().unwrap(), 2);
        assert_eq!(result.unwrap(), b"ok");
    }

    #[test]
    fn retries_after_request_timeout_status() {
        let (url, server) = start_server(vec![
            TestResponse::new(408, "retry"),
            TestResponse::new(200, "ok"),
        ]);
        let result = request_body(&url);
        assert_eq!(server.join().unwrap(), 2);
        assert_eq!(result.unwrap(), b"ok");
    }

    #[test]
    fn honors_retry_after_seconds() {
        let (url, server) = start_server(vec![
            TestResponse::new(503, "retry").with_retry_after("1"),
            TestResponse::new(200, "ok"),
        ]);
        let started = Instant::now();
        let result = request_body(&url);
        let elapsed = started.elapsed();
        assert_eq!(server.join().unwrap(), 2);
        assert_eq!(result.unwrap(), b"ok");
        assert!(
            elapsed >= Duration::from_millis(900),
            "Retry-After was not honored: {elapsed:?}"
        );
    }

    #[test]
    fn retries_when_response_body_is_truncated() {
        let (url, server) = start_server(vec![
            TestResponse::new(200, "partial").with_content_length(100),
            TestResponse::new(200, "complete"),
        ]);
        let result = request_body(&url);
        assert_eq!(server.join().unwrap(), 2);
        assert_eq!(result.unwrap(), b"complete");
    }

    #[test]
    fn makes_no_more_than_three_attempts() {
        let (url, server) = start_server(vec![
            TestResponse::new(503, "first"),
            TestResponse::new(503, "second"),
            TestResponse::new(503, "third"),
        ]);
        let result = request_body(&url);
        assert_eq!(server.join().unwrap(), 3);
        assert!(result.is_err());
    }

    #[test]
    fn does_not_retry_not_found() {
        let (url, server) = start_server(vec![TestResponse::new(404, "not found")]);
        let result = request_body(&url);
        assert_eq!(server.join().unwrap(), 1);
        assert!(result.is_err());
    }
}
