use clap::Args;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone)]
pub enum ManjaroBranch {
    Stable,
    Testing,
    Unstable,
}
impl FromStr for ManjaroBranch {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stable" => Ok(ManjaroBranch::Stable),
            "testing" => Ok(ManjaroBranch::Testing),
            "unstable" => Ok(ManjaroBranch::Unstable),
            _ => Err("could not parse branch"),
        }
    }
}

impl fmt::Display for ManjaroBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            ManjaroBranch::Stable => "stable",
            ManjaroBranch::Testing => "testing",
            ManjaroBranch::Unstable => "unstable",
        };
        write!(f, "{}", repr)
    }
}

#[derive(Debug, Clone, Args)]
pub struct ManjaroTarget {
    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// Max acceptable delay in seconds since the last time a mirror has been
    /// synced
    #[arg(env = "RATE_MIRRORS_MAX_DELAY", long, default_value = "86400")]
    pub max_delay: i64,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "extra/x86_64/extra.files",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,

    /// Select mirrors providing a particular branch;
    ///   choices: stable, testing, unstable
    #[arg(
        env = "RATE_MIRRORS_BRANCH",
        long,
        default_value = "stable",
        verbatim_doc_comment
    )]
    pub branch: ManjaroBranch,
}
