use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct OpenBSDTarget {
    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "/snapshots/ports.tar.gz",
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

    /// Either url or path to OpenBSD ftplist file
    #[arg(
        env = "RATE_MIRRORS_MIRROR_SOURCE",
        long,
        default_value = "https://ftp.openbsd.org/pub/OpenBSD/ftplist",
        verbatim_doc_comment
    )]
    pub mirror_source: String,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,
}
