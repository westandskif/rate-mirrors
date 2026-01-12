use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct BlackArchTarget {
    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
        #[arg(
            env = "RATE_MIRRORS_BASE_PATH",
            long,
            default_value = "blackarch/os/x86_64/blackarch",
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
