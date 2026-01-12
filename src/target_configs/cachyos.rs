use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct CachyOSTarget {
    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

        /// Base path to repository resources (used for both speed test .files and freshness .db)
        ///   Example: "x86_64/cachyos/cachyos"
        #[arg(
            env = "RATE_MIRRORS_BASE_PATH",
            long,
            default_value = "x86_64/cachyos/cachyos",
            verbatim_doc_comment
        )]
        pub base_path: String,

    /// Architecture
    #[arg(env = "RATE_MIRRORS_ARCH", long, default_value = "auto")]
    pub arch: String,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,
}
