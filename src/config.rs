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
use ambassador::{delegatable_trait, Delegate};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fmt;
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

    /// Per-mirror: sigma to mean speed ratio
    ///
    ///   1.0 -- 68% probability (1 sigma), no 100% error
    ///   0.5 -- 68% probability (1 sigma), no 50% error;
    ///   0.25 -- 68% probability (1 sigma), no 25% error;
    ///   0.125 -- 95% probability (2 sigmas), no 25% error;
    ///   0.0625 -- 95% probability (2 sigmas), no 12.5% error:
    #[arg(
        env = "RATE_MIRRORS_EPS",
        long,
        default_value = "0.0625",
        verbatim_doc_comment
    )]
    pub eps: f64,

    /// Per-mirror: after min measurement time elapsed, check such number of
    /// subsequently downloaded data chunks whether speed variations are less
    /// then "eps"
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
    #[arg(
        env = "RATE_MIRRORS_EXCLUDE_COUNTRIES",
        long = "exclude-countries",
        name = "country-codes",
        verbatim_doc_comment
    )]
    pub exclude_countries: Option<String>,

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
}

impl Config {
    pub fn new() -> Self {
        let mut config = Self::parse();
        config.excluded_countries_set = config
            .exclude_countries
            .as_ref()
            .map(|s| {
                s.split(',')
                    .map(|c| c.trim().to_ascii_lowercase())
                    .filter(|c| !c.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        config
    }

    pub fn is_country_excluded(&self, code: &str) -> bool {
        self.excluded_countries_set
            .contains(&code.to_ascii_lowercase())
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

fn convert_reqwest_error(e: reqwest::Error, url: &str) -> AppError {
    if e.is_timeout() {
        AppError::RequestTimeout(url.to_string())
    } else {
        AppError::RequestError(format!("failed to connect to {}: {}", url, e))
    }
}

pub fn fetch_json<T: DeserializeOwned>(url: &str, timeout_ms: u64) -> Result<T, AppError> {
    Runtime::new().unwrap().block_on(async {
        let response = reqwest::Client::new()
            .get(url)
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| convert_reqwest_error(e, url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::HttpError {
                status: status.as_u16(),
                url: url.to_string(),
            });
        }

        response.json::<T>().await.map_err(|e| {
            AppError::RequestError(format!("failed to decode JSON from {}: {}", url, e))
        })
    })
}

pub fn fetch_text(url: &str, timeout_ms: u64) -> Result<String, AppError> {
    Runtime::new().unwrap().block_on(async {
        let response = reqwest::Client::new()
            .get(url)
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| convert_reqwest_error(e, url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::HttpError {
                status: status.as_u16(),
                url: url.to_string(),
            });
        }

        response.text_with_charset("utf-8").await.map_err(|e| {
            AppError::RequestError(format!("failed to read response from {}: {}", url, e))
        })
    })
}
