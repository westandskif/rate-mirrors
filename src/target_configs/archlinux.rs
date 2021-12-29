use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, Clone)]
pub enum ArchMirrorsSortingStrategy {
    DelayAsc,
    DelayDesc,
    Random,
    ScoreAsc,
    ScoreDesc,
}
impl FromStr for ArchMirrorsSortingStrategy {
    type Err = &'static str;
    fn from_str(strategy: &str) -> Result<Self, Self::Err> {
        match strategy {
            "delay_asc" => Ok(ArchMirrorsSortingStrategy::DelayAsc),
            "delay_desc" => Ok(ArchMirrorsSortingStrategy::DelayDesc),
            "random" => Ok(ArchMirrorsSortingStrategy::Random),
            "score_asc" => Ok(ArchMirrorsSortingStrategy::ScoreAsc),
            "score_desc" => Ok(ArchMirrorsSortingStrategy::ScoreDesc),
            _ => Err("could not parse strategy"),
        }
    }
}
#[derive(Debug, Clone, StructOpt)]
pub struct ArchTarget {
    /// Minimum mirror sync completion percentage, in a range of 0-1.
    ///   If this is below 1, the mirror synchronization is in progress and it's
    ///   best to filter out such mirrors [default: 1]
    #[structopt(long = "completion", default_value = "1", verbatim_doc_comment)]
    pub completion: f64,

    /// Max acceptable delay in seconds since the last time a mirror has been
    /// synced
    #[structopt(long = "max-delay", default_value = "86400")]
    pub max_delay: u64,

    /// Mirrors sorting strategy, one of:
    ///   score_asc, score_desc, delay_asc, delay_desc, random
    /// [default: score_asc] (lower is better)
    ///   see https://archlinux.org/mirrors/status/ for score definition
    #[structopt(
        long = "sort-mirrors-by",
        verbatim_doc_comment,
        default_value = "score_asc"
    )]
    pub sort_mirrors_by: ArchMirrorsSortingStrategy,

    /// Path to be joined to a mirror url and used for speed testing
    ///   the file should be big enough to allow for testing high
    ///   speed connections
    #[structopt(
        long = "path-to-test",
        default_value = "community/os/x86_64/community.files",
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
