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
use crate::target_configs::fetch::FetchArgs;
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
use std::time::{Duration, Instant};
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
    #[error("{last_error} (gave up after {attempts} attempt(s) in {elapsed:.1?}: {stop_reason})")]
    RetriesFailed {
        last_error: Box<AppError>,
        attempts: usize,
        elapsed: Duration,
        stop_reason: StopReason,
    },
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

pub fn parse_positive_usize(s: &str) -> Result<usize, String> {
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

fn build_client(builder: reqwest::ClientBuilder) -> Result<reqwest::Client, AppError> {
    builder
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME").replace('_', "-"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| AppError::RequestError(format!("failed to build HTTP client: {}", e)))
}

pub fn default_client_builder() -> Result<reqwest::Client, AppError> {
    build_client(reqwest::Client::builder())
}

/// Time constants of the mirror-list fetch retry loop; tests pass a scaled-down copy.
struct RetryTiming {
    /// Delay before the second attempt, doubled for every further one.
    backoff_base: Duration,
    connect_timeout: Duration,
    /// Ends an attempt that receives no headers, or no body data, for this long.
    read_timeout: Duration,
    /// No retry is started unless at least this much of the budget is left after the wait.
    min_remaining: Duration,
    /// A longer `Retry-After` ends retrying instead of being waited out.
    max_retry_after: Duration,
}

const RETRY_TIMING: RetryTiming = RetryTiming {
    backoff_base: Duration::from_millis(250),
    connect_timeout: Duration::from_secs(5),
    read_timeout: Duration::from_secs(10),
    min_remaining: Duration::from_secs(1),
    max_retry_after: Duration::from_secs(5),
};

#[derive(Debug, PartialEq)]
pub enum StopReason {
    AttemptsExhausted,
    DeadlineReached,
    RetryAfterTooLong(Duration),
    NotRetryable,
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopReason::AttemptsExhausted => write!(f, "no attempts left"),
            StopReason::DeadlineReached => write!(f, "fetch-mirrors-timeout reached"),
            StopReason::RetryAfterTooLong(delay) => {
                write!(f, "server asked to retry after {:?}", delay)
            }
            StopReason::NotRetryable => write!(f, "the error is not retryable"),
        }
    }
}

fn request_error(error: reqwest::Error, url: &str) -> AppError {
    if error.is_timeout() {
        AppError::RequestTimeout(url.to_string())
    } else {
        AppError::RequestError(format!("failed to fetch {}: {}", url, root_cause(&error)))
    }
}

/// reqwest's own message only names the request; the innermost source says what failed.
fn root_cause(error: &reqwest::Error) -> String {
    let mut cause: &dyn std::error::Error = error;
    while let Some(source) = cause.source() {
        cause = source;
    }
    cause.to_string()
}

/// Every error `send()` can return for a well-formed request: connect failures, timeouts and
/// connections dropped before headers.
fn is_retryable_request_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let value = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    chrono::DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|date| {
            let remaining = date.with_timezone(&chrono::Utc) - chrono::Utc::now();
            Duration::from_millis(remaining.num_milliseconds().max(0) as u64)
        })
}

/// Delay before the attempt following failed attempt `attempt` (0-based), or why there is none.
/// The check happens before sleeping, so no wait ever crosses the deadline.
fn next_retry_delay(
    attempt: usize,
    attempts: usize,
    retry_after: Option<Duration>,
    now: Instant,
    deadline: Instant,
    timing: &RetryTiming,
) -> Result<Duration, StopReason> {
    if attempt + 1 >= attempts {
        return Err(StopReason::AttemptsExhausted);
    }
    let backoff = timing.backoff_base.saturating_mul(1 << attempt.min(20));
    let delay = match retry_after {
        Some(retry_after) if retry_after > timing.max_retry_after => {
            return Err(StopReason::RetryAfterTooLong(retry_after));
        }
        Some(retry_after) => retry_after.max(backoff),
        None => backoff,
    };
    if now + delay + timing.min_remaining >= deadline {
        return Err(StopReason::DeadlineReached);
    }
    Ok(delay)
}

/// GETs `url` until `consume_response` succeeds, retrying transient failures within one
/// `fetch_mirrors_timeout` budget shared by all attempts.
async fn request_with_retry<T, F, Fut>(
    url: &str,
    fetch: &FetchArgs,
    timing: &RetryTiming,
    tx_progress: &mpsc::Sender<String>,
    consume_response: F,
) -> Result<T, AppError>
where
    F: Fn(reqwest::Response) -> Fut,
    Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
{
    let client = build_client(
        reqwest::Client::builder()
            .connect_timeout(timing.connect_timeout)
            .read_timeout(timing.read_timeout),
    )?;
    let started = Instant::now();
    let deadline = started + Duration::from_millis(fetch.fetch_mirrors_timeout);
    let attempts = fetch.fetch_mirrors_attempts;

    let mut attempt = 0;
    let (last_error, stop_reason) = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        // A retryable failure: the error to return if retrying stops, the reason to log,
        // and the server's Retry-After.
        let (error, reason, retry_after) = match client.get(url).timeout(remaining).send().await {
            Err(error) if is_retryable_request_error(&error) => {
                let reason = root_cause(&error);
                (request_error(error, url), reason, None)
            }
            Err(error) => break (request_error(error, url), StopReason::NotRetryable),
            Ok(response) => {
                let status = response.status();
                let http_error = AppError::HttpError {
                    status: status.as_u16(),
                    url: url.to_string(),
                };
                if is_retryable_status(status) {
                    let retry_after = parse_retry_after(response.headers());
                    (http_error, format!("HTTP {}", status.as_u16()), retry_after)
                } else if !status.is_success() {
                    break (http_error, StopReason::NotRetryable);
                } else {
                    // Only the body transfer is retried; callers parse it after this returns.
                    match consume_response(response).await {
                        Ok(value) => return Ok(value),
                        Err(error) => {
                            let reason = root_cause(&error);
                            (request_error(error, url), reason, None)
                        }
                    }
                }
            }
        };

        let delay = match next_retry_delay(
            attempt,
            attempts,
            retry_after,
            Instant::now(),
            deadline,
            timing,
        ) {
            Ok(delay) => delay,
            Err(stop_reason) => break (error, stop_reason),
        };
        attempt += 1;
        // A closed receiver means the run is ending; the retry goes ahead regardless.
        let _ = tx_progress.send(format!(
            "RETRY {}/{} {}: {}, waiting {:?}",
            attempt + 1,
            attempts,
            url,
            reason,
            delay
        ));
        tokio::time::sleep(delay).await;
    };

    // A first attempt failing for good keeps its plain error; anything after a retry says why
    // retrying ended.
    if attempt == 0 && stop_reason == StopReason::NotRetryable {
        return Err(last_error);
    }
    Err(AppError::RetriesFailed {
        last_error: Box::new(last_error),
        attempts: attempt + 1,
        elapsed: started.elapsed(),
        stop_reason,
    })
}

pub fn fetch_json<T: DeserializeOwned>(
    url: &str,
    fetch: &FetchArgs,
    tx_progress: &mpsc::Sender<String>,
) -> Result<T, AppError> {
    let runtime = Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let body = request_with_retry(
            url,
            fetch,
            &RETRY_TIMING,
            tx_progress,
            |response| async move { response.bytes().await },
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

pub fn fetch_text(
    url: &str,
    fetch: &FetchArgs,
    tx_progress: &mpsc::Sender<String>,
) -> Result<String, AppError> {
    let runtime = Runtime::new().unwrap();
    let result = runtime.block_on(request_with_retry(
        url,
        fetch,
        &RETRY_TIMING,
        tx_progress,
        |response| async move { response.text_with_charset("utf-8").await },
    ));
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

/// Fetches text content from either a remote URL or a local file path.
/// URLs are detected via `Url::parse`; everything else is treated as a filesystem path.
/// Used by mirror-list and mirror-source options that can point at a URL or a file.
pub fn fetch_text_or_file(
    path_or_url: &str,
    fetch: &FetchArgs,
    tx_progress: &mpsc::Sender<String>,
) -> Result<String, AppError> {
    if Url::parse(path_or_url).is_ok() {
        fetch_text(path_or_url, fetch, tx_progress)
    } else {
        fs::read_to_string(path_or_url)
            .map_err(|e| AppError::RequestError(format!("failed to read mirror source: {}", e)))
    }
}

/// Same as `fetch_text_or_file` but deserializes the result as JSON.
/// Supports both remote status endpoints and local JSON files for the mirror list source.
pub fn fetch_json_or_file<T: DeserializeOwned>(
    path_or_url: &str,
    fetch: &FetchArgs,
    tx_progress: &mpsc::Sender<String>,
) -> Result<T, AppError> {
    if Url::parse(path_or_url).is_ok() {
        fetch_json(path_or_url, fetch, tx_progress)
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
    use super::{
        AppError, FetchArgs, RETRY_TIMING, RetryTiming, StopReason, is_retryable_status,
        next_retry_delay, parse_retry_after, request_with_retry,
    };
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    /// Production timing scaled down so stalls and backoffs cost milliseconds.
    const TEST_TIMING: RetryTiming = RetryTiming {
        backoff_base: Duration::from_millis(1),
        connect_timeout: Duration::from_millis(100),
        read_timeout: Duration::from_millis(50),
        min_remaining: Duration::from_millis(5),
        max_retry_after: Duration::from_millis(50),
    };

    #[test]
    fn retries_only_transient_statuses() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_status(status.try_into().unwrap()), "{status}");
        }
        for status in [200, 400, 404, 501, 505, 511] {
            assert!(!is_retryable_status(status.try_into().unwrap()), "{status}");
        }
    }

    #[test]
    fn retry_after_sets_delay_up_to_cap() {
        let retry_after = |value: &str| {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(reqwest::header::RETRY_AFTER, value.parse().unwrap());
            parse_retry_after(&headers)
        };
        let now = Instant::now();
        let delay = |value| {
            next_retry_delay(
                0,
                3,
                value,
                now,
                now + Duration::from_secs(30),
                &RETRY_TIMING,
            )
        };

        assert_eq!(delay(retry_after("3")), Ok(Duration::from_secs(3)));
        // Zero and past dates fall back to the backoff instead of retrying immediately.
        assert_eq!(delay(retry_after("0")), Ok(Duration::from_millis(250)));
        assert_eq!(
            delay(retry_after("Wed, 21 Oct 2015 07:28:00 GMT")),
            Ok(Duration::from_millis(250))
        );
        assert_eq!(
            delay(retry_after("6")),
            Err(StopReason::RetryAfterTooLong(Duration::from_secs(6)))
        );
        assert!(matches!(
            delay(retry_after("Fri, 01 Jan 2100 00:00:00 GMT")),
            Err(StopReason::RetryAfterTooLong(_))
        ));
        assert_eq!(delay(None), Ok(Duration::from_millis(250)));
    }

    #[test]
    fn stops_before_a_wait_that_would_cross_the_deadline() {
        let now = Instant::now();
        let deadline = now + Duration::from_secs(10);
        let delay = |attempt, attempts, now| {
            next_retry_delay(attempt, attempts, None, now, deadline, &RETRY_TIMING)
        };

        assert_eq!(delay(1, 3, now), Ok(Duration::from_millis(500)));
        assert_eq!(delay(2, 3, now), Err(StopReason::AttemptsExhausted));
        assert_eq!(delay(0, 1, now), Err(StopReason::AttemptsExhausted));
        // 500 ms backoff + 1 s minimum remaining no longer fit.
        assert_eq!(
            delay(1, 3, deadline - Duration::from_millis(1400)),
            Err(StopReason::DeadlineReached)
        );
    }

    /// What the test server does with one accepted connection, after reading the request.
    enum Conn {
        Respond {
            status: u16,
            body: &'static str,
            content_length: Option<usize>,
        },
        /// Closes without sending headers.
        Close,
        /// Keeps the connection silent for a while, then closes it without headers.
        HoldThenClose(Duration),
        /// Sends nothing, keeps the connection open until the server stops.
        Stall,
        /// Sends headers and a part of the body, then keeps the connection open silently.
        PartialBodyThenStall,
    }

    fn respond(status: u16, body: &'static str) -> Conn {
        Conn::Respond {
            status,
            body,
            content_length: None,
        }
    }

    struct TestServer {
        url: String,
        stop: Arc<AtomicBool>,
        handle: JoinHandle<usize>,
    }

    impl TestServer {
        /// Serves `script` connection by connection; connections beyond it are closed.
        fn start(script: Vec<Conn>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}/", listener.local_addr().unwrap());
            let stop = Arc::new(AtomicBool::new(false));
            let stop_flag = stop.clone();

            let handle = thread::spawn(move || {
                let mut script = script.into_iter();
                let mut held = Vec::new();
                let mut connections = 0;
                while !stop_flag.load(Ordering::SeqCst) {
                    let mut stream = match listener.accept() {
                        Ok((stream, _)) => stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        Err(error) => panic!("accept failed: {error}"),
                    };
                    connections += 1;
                    stream.set_nonblocking(false).unwrap();
                    read_request(&mut stream);
                    match script.next().unwrap_or(Conn::Close) {
                        Conn::Respond {
                            status,
                            body,
                            content_length,
                        } => {
                            let length = content_length.unwrap_or(body.len());
                            write!(
                                stream,
                                "HTTP/1.1 {status} Test\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
                            )
                            .unwrap();
                        }
                        Conn::Close => {}
                        Conn::HoldThenClose(duration) => thread::sleep(duration),
                        Conn::Stall => held.push(stream),
                        Conn::PartialBodyThenStall => {
                            write!(
                                stream,
                                "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\npartial"
                            )
                            .unwrap();
                            held.push(stream);
                        }
                    }
                }
                connections
            });

            TestServer { url, stop, handle }
        }

        /// Stops the server and returns how many connections it accepted.
        fn connections(self) -> usize {
            self.stop.store(true, Ordering::SeqCst);
            self.handle.join().unwrap()
        }
    }

    /// Reads the whole request head, so closing the connection never resets it.
    fn read_request(stream: &mut TcpStream) {
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).unwrap();
            assert!(
                read > 0,
                "connection closed before the request was complete"
            );
            request.extend_from_slice(&buffer[..read]);
        }
    }

    struct Outcome {
        result: Result<Vec<u8>, AppError>,
        connections: usize,
        progress: Vec<String>,
    }

    fn fetch(script: Vec<Conn>, budget_ms: u64, attempts: usize, timing: &RetryTiming) -> Outcome {
        let server = TestServer::start(script);
        let (tx, rx) = mpsc::channel();
        let fetch = FetchArgs {
            fetch_mirrors_timeout: budget_ms,
            fetch_mirrors_attempts: attempts,
        };
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime
            .block_on(request_with_retry(
                &server.url,
                &fetch,
                timing,
                &tx,
                |response| async move { response.bytes().await },
            ))
            .map(|body| body.to_vec());
        Outcome {
            result,
            connections: server.connections(),
            progress: rx.try_iter().collect(),
        }
    }

    #[test]
    fn retries_service_unavailable_and_logs_the_retry() {
        let outcome = fetch(
            vec![respond(503, "retry"), respond(200, "ok")],
            2000,
            3,
            &TEST_TIMING,
        );
        assert_eq!(outcome.result.unwrap(), b"ok");
        assert_eq!(outcome.connections, 2);
        assert_eq!(outcome.progress.len(), 1);
        assert!(
            outcome.progress[0].starts_with("RETRY 2/3 http://127.0.0.1:")
                && outcome.progress[0].ends_with(": HTTP 503, waiting 1ms"),
            "{:?}",
            outcome.progress
        );
    }

    #[test]
    fn retries_connection_closed_before_headers() {
        let outcome = fetch(vec![Conn::Close, respond(200, "ok")], 2000, 3, &TEST_TIMING);
        assert_eq!(outcome.result.unwrap(), b"ok");
        assert_eq!(outcome.connections, 2);
    }

    #[test]
    fn retries_truncated_body() {
        let truncated = Conn::Respond {
            status: 200,
            body: "partial",
            content_length: Some(100),
        };
        let outcome = fetch(
            vec![truncated, respond(200, "complete")],
            2000,
            3,
            &TEST_TIMING,
        );
        assert_eq!(outcome.result.unwrap(), b"complete");
        assert_eq!(outcome.connections, 2);
    }

    #[test]
    fn retries_stalled_body() {
        let outcome = fetch(
            vec![Conn::PartialBodyThenStall, respond(200, "complete")],
            2000,
            3,
            &TEST_TIMING,
        );
        assert_eq!(outcome.result.unwrap(), b"complete");
        assert_eq!(outcome.connections, 2);
    }

    #[test]
    fn retries_stalled_headers() {
        let outcome = fetch(vec![Conn::Stall, respond(200, "ok")], 2000, 3, &TEST_TIMING);
        assert_eq!(outcome.result.unwrap(), b"ok");
        assert_eq!(outcome.connections, 2);
    }

    #[test]
    fn does_not_retry_not_found() {
        let outcome = fetch(vec![respond(404, "not found")], 2000, 3, &TEST_TIMING);
        assert!(matches!(
            outcome.result,
            Err(AppError::HttpError { status: 404, .. })
        ));
        assert_eq!(outcome.connections, 1);
        assert!(outcome.progress.is_empty());
    }

    #[test]
    fn reports_retries_before_a_non_retryable_status() {
        let outcome = fetch(
            vec![respond(503, "retry"), respond(404, "not found")],
            2000,
            3,
            &TEST_TIMING,
        );
        assert_eq!(outcome.connections, 2);
        match outcome.result {
            Err(AppError::RetriesFailed {
                last_error,
                attempts: 2,
                stop_reason: StopReason::NotRetryable,
                ..
            }) => assert!(matches!(
                *last_error,
                AppError::HttpError { status: 404, .. }
            )),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn makes_no_more_than_the_configured_attempts() {
        let outcome = fetch(
            vec![respond(503, "1"), respond(503, "2"), respond(503, "3")],
            2000,
            3,
            &TEST_TIMING,
        );
        assert_eq!(outcome.connections, 3);
        assert_eq!(outcome.progress.len(), 2);
        match outcome.result {
            Err(AppError::RetriesFailed {
                last_error,
                attempts: 3,
                stop_reason: StopReason::AttemptsExhausted,
                ..
            }) => assert!(matches!(
                *last_error,
                AppError::HttpError { status: 503, .. }
            )),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn attempts_share_one_deadline() {
        // A read timeout longer than the budget, so only the request timeout can end attempt 2.
        let timing = RetryTiming {
            read_timeout: Duration::from_secs(10),
            ..TEST_TIMING
        };
        let budget = Duration::from_millis(300);
        let outcome = fetch(
            vec![Conn::HoldThenClose(budget * 6 / 10), Conn::Stall],
            budget.as_millis() as u64,
            3,
            &timing,
        );
        assert_eq!(outcome.connections, 2);
        match outcome.result {
            Err(AppError::RetriesFailed {
                attempts: 2,
                stop_reason: StopReason::DeadlineReached,
                elapsed,
                ..
            }) => {
                // A per-attempt timeout would end attempt 2 only at about 1.6 × budget.
                assert!(
                    elapsed >= budget && elapsed < budget * 13 / 10,
                    "{elapsed:?}"
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
