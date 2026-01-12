use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct EndeavourOSTarget {
    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// Max time to fetch mirror version
    #[arg(
        env = "RATE_MIRRORS_VERSION_MIRROR_TIMEOUT",
        long,
        default_value = "3000"
    )]
    pub version_mirror_timeout: u64,

    /// Max number of concurrent requests to fetch mirror versions
    #[arg(
        env = "RATE_MIRRORS_VERSION_MIRROR_CONCURRENCY",
        long,
        default_value = "40"
    )]
    pub version_mirror_concurrency: usize,

    /// Either url or path to EndeavourOS mirror list file
    #[arg(
        env = "RATE_MIRRORS_MIRROR_LIST_FILE",
        long,
        default_value = "https://raw.githubusercontent.com/endeavouros-team/PKGBUILDS/master/endeavouros-mirrorlist/endeavouros-mirrorlist",
        verbatim_doc_comment
    )]
    pub mirror_list_file: String,

    /// Base path to repository resources (used for both speed test .files and freshness .db)
    ///   Example: "endeavouros/x86_64/endeavouros"
    #[arg(
        env = "RATE_MIRRORS_BASE_PATH",
        long,
        default_value = "endeavouros/x86_64/endeavouros",
        verbatim_doc_comment
    )]
    pub base_path: String,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,
}
