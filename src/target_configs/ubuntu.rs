use super::debian::SourceListEntriesOpts;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct UbuntuTarget {
    #[structopt(flatten)]
    pub source_list_opts: SourceListEntriesOpts,

    /// Max acceptable delay in seconds since the last time a mirror has been synced
    #[structopt(long = "max-delay", default_value = "86400")]
    pub max_delay: u64,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[structopt(
        long = "path-to-test",
        default_value = "ls-lR.gz",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// Fetch list of mirrors timeout in milliseconds
    #[structopt(long = "fetch-mirrors-timeout", default_value = "15000")]
    pub fetch_mirrors_timeout: u64,

    /// comment prefix to use when outputting
    #[structopt(long = "comment-prefix", default_value = "# ")]
    pub comment_prefix: String,
}
