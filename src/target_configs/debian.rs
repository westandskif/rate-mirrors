use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct SourceListEntriesOpts {
    /// The deb type references a typical two-level Debian archive, distribution/component
    /// deb-src type references a Debian distribution's source code
    #[structopt(long = "types", default_value = "deb", verbatim_doc_comment)]
    pub types: Vec<String>,

    /// options specified to modify which source is accessed and how data is acquired from it
    #[structopt(long = "options")]
    pub options: Vec<String>,

    /// suite name like stable or testing or a codename like jessie or stretch
    #[structopt(long = "suites", required = true)]
    pub suites: Vec<String>,

    /// archive components like main, restricted, universe and multiverse
    #[structopt(long = "components", default_value = "main")]
    pub components: Vec<String>,
}

#[derive(Debug, Clone, StructOpt)]
pub struct DebianTarget {
    #[structopt(flatten)]
    pub source_list_opts: SourceListEntriesOpts,

    /// Max acceptable delay in seconds since the last time a mirror has been synced
    #[structopt(long = "max-delay", default_value = "86400")]
    pub max_delay: i64,

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
