use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ArcoLinuxTarget {
    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "arcolinux_repo_3party/x86_64/arcolinux_repo_3party.files",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// Path to be joined to a gitlab-based mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_GITLAB_PATH_TO_TEST",
        long,
        default_value = "arcolinux_repo_3party/-/raw/main/x86_64/arcolinux_repo_3party.files",
        verbatim_doc_comment
    )]
    pub gitlab_path_to_test: String,

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
