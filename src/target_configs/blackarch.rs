use crate::target_configs::fetch::FetchArgs;
use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct BlackArchTarget {
    #[command(flatten)]
    pub fetch: FetchArgs,

    /// Either url or path to BlackArch mirror list file
    #[arg(
        env = "RATE_MIRRORS_MIRROR_SOURCE",
        long,
        default_value = "https://raw.githubusercontent.com/BlackArch/blackarch/master/mirror/mirror.lst",
        verbatim_doc_comment
    )]
    pub mirror_source: String,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "blackarch/os/x86_64/blackarch.files",
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
