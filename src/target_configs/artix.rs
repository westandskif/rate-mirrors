use crate::target_configs::archlinux::ArchMirrorsSortingStrategy;
use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ArtixTarget {
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
    ///   see https://status.artixlinux.org/mirrors/status/ for score definition
    #[arg(
        env = "RATE_MIRRORS_SORT_MIRRORS_BY",
        long,
        verbatim_doc_comment,
        default_value = "score_asc"
    )]
    pub sort_mirrors_by: ArchMirrorsSortingStrategy,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "world/os/x86_64/world.files",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// Either url or path to Artix mirrors status JSON file
    #[arg(
        env = "RATE_MIRRORS_MIRROR_SOURCE",
        long,
        default_value = "https://status.artixlinux.org/mirrors/status/json/",
        conflicts_with = "fetch_first_tier_only",
        verbatim_doc_comment
    )]
    pub mirror_source: String,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,

    /// Fetch only list of tier 1 mirrors
    #[arg(env = "RATE_MIRRORS_FETCH_FIRST_TIER_ONLY", long)]
    pub fetch_first_tier_only: bool,
}
