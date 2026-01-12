use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ArtixTarget {
    /// Base path to repository resources (used for both speed test .files and freshness .db)
    ///   Example: "world/os/x86_64/world"
    #[arg(
        env = "RATE_MIRRORS_BASE_PATH",
        long,
        default_value = "world/os/x86_64/world",
        verbatim_doc_comment
    )]
    pub base_path: String,

    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,
}
