use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ArcharmTarget {
    /// Fetch list of mirrors timeout in milliseconds
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "15000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// Either url or path to Arch Linux ARM mirror list file
    #[arg(
        env = "RATE_MIRRORS_MIRROR_LIST_FILE",
        long,
        default_value = "https://raw.githubusercontent.com/archlinuxarm/PKGBUILDs/master/core/pacman-mirrorlist/mirrorlist",
        verbatim_doc_comment
    )]
    pub mirror_list_file: String,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "aarch64/extra/extra.files",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// Architecture
    #[arg(env = "RATE_MIRRORS_ARCH", long, default_value = "auto")]
    pub arch: String,

    /// comment prefix to use when outputting
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,
}
