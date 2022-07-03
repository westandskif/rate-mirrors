use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
pub struct ArcharmTarget {
    /// Fetch list of mirrors timeout in milliseconds
    #[structopt(long = "fetch-mirrors-timeout", default_value = "15000")]
    pub fetch_mirrors_timeout: u64,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[structopt(
        long = "path-to-test",
        default_value = "aarch64/community/community.files",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// Architecture
    #[structopt(long = "arch", default_value = "auto")]
    pub arch: String,

    /// comment prefix to use when outputting
    #[structopt(long = "comment-prefix", default_value = "# ")]
    pub comment_prefix: String,
}
