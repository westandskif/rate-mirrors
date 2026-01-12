use clap::Args;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum ArchMirrorsSortingStrategy {
    DelayAsc,
    DelayDesc,
    Random,
    ScoreAsc,
    ScoreDesc,
}
impl FromStr for ArchMirrorsSortingStrategy {
    type Err = &'static str;
    fn from_str(strategy: &str) -> Result<Self, Self::Err> {
        match strategy {
            "delay_asc" => Ok(ArchMirrorsSortingStrategy::DelayAsc),
            "delay_desc" => Ok(ArchMirrorsSortingStrategy::DelayDesc),
            "random" => Ok(ArchMirrorsSortingStrategy::Random),
            "score_asc" => Ok(ArchMirrorsSortingStrategy::ScoreAsc),
            "score_desc" => Ok(ArchMirrorsSortingStrategy::ScoreDesc),
            _ => Err("could not parse strategy"),
        }
    }
}
#[derive(Debug, Clone, Args)]
pub struct ArchTarget {
    /// Minimum mirror sync completion percentage, in a range of 0-1.
    ///   If this is below 1, the mirror synchronization is in progress and it's
    ///   best to filter out such mirrors [default: 1]
    #[arg(
        env = "RATE_MIRRORS_COMPLETION",
        long,
        default_value = "1",
        verbatim_doc_comment
    )]
    pub completion: f64,

    /// Max acceptable delay in seconds since the last time a mirror has been
    /// synced
    #[arg(env = "RATE_MIRRORS_MAX_DELAY", long, default_value = "86400")]
    pub max_delay: i64,

    /// Mirrors sorting strategy, one of:
    ///   score_asc, score_desc, delay_asc, delay_desc, random
    /// [default: score_asc] (lower is better)
    ///   see https://archlinux.org/mirrors/status/ for score definition
    #[arg(
        env = "RATE_MIRRORS_SORT_MIRRORS_BY",
        long,
        verbatim_doc_comment,
        default_value = "score_asc"
    )]
    pub sort_mirrors_by: ArchMirrorsSortingStrategy,

        /// Base path to repository resources (used for both speed test .files and freshness .db)
        ///   Example: "extra/os/x86_64/extra"
        #[arg(
            env = "RATE_MIRRORS_BASE_PATH",
            long,
            default_value = "extra/os/x86_64/extra",
            verbatim_doc_comment
        )]
        pub base_path: String,

    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "30000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,

    /// Fetch only list of tier 1 mirrors
    #[arg(env = "RATE_MIRRORS_FETCH_FIRST_TIER_ONLY", long)]
    pub fetch_first_tier_only: bool,
}
