use std::fmt::Debug;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug)]
pub enum MirrorsSortingStrategy {
    DelayAsc,
    DelayDesc,
    ScoreAsc,
    ScoreDesc,
    Random,
}
impl FromStr for MirrorsSortingStrategy {
    type Err = &'static str;
    fn from_str(strategy: &str) -> Result<Self, Self::Err> {
        match strategy {
            "delay_asc" => Ok(MirrorsSortingStrategy::DelayAsc),
            "delay_desc" => Ok(MirrorsSortingStrategy::DelayDesc),
            "score_asc" => Ok(MirrorsSortingStrategy::ScoreAsc),
            "score_desc" => Ok(MirrorsSortingStrategy::ScoreDesc),
            "random" => Ok(MirrorsSortingStrategy::Random),
            _ => Err("failed to parse sorting strategy"),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "rate-arch-mirrors config")]
/// Usually default options should work
pub struct Config {
    /// Minimum mirror sync completion percentage, in a range of 0-1.
    ///   If this is below 1, the mirror synchronization is in progress and it's
    ///   best to filter out such mirrors [default: 1]
    #[structopt(long = "completion", default_value = "1", verbatim_doc_comment)]
    pub completion: f64,
    /// Max acceptable delay in seconds since the last time a mirror has been
    /// synced
    #[structopt(long = "max-delay", default_value = "86400")]
    pub max_delay: u64,
    /// Fetch list of mirrors timeout in milliseconds
    #[structopt(long = "fetch-mirrors-timeout", default_value = "15000")]
    pub fetch_mirrors_timeout: u64,
    /// Per-mirror speed test timeout in milliseconds
    #[structopt(long = "per-mirror-timeout", default_value = "1500")]
    pub per_mirror_timeout: u64,
    /// Minimum downloading time, required to measure mirror speed,
    /// in milliseconds
    #[structopt(long = "min-per-mirror", default_value = "300")]
    pub min_per_mirror: u64,
    /// An in-mirror file path to be used for speed test (the file should be
    ///   big enough to allow for testing high speed connections)
    #[structopt(
        long = "mirror-path",
        default_value = "community/os/x86_64/community.files",
        verbatim_doc_comment
    )]
    pub path_to_test: String,
    /// Minimum number of bytes to be downloaded,
    /// required to measure mirror speed
    #[structopt(long = "min-bytes-per-mirror", default_value = "70000")]
    pub min_bytes_per_mirror: usize,
    /// Per-mirror: sigma to mean speed ratio
    ///
    ///   1.0 -- 68% probability (1 sigma), no 100% error
    ///   0.5 -- 68% probability (1 sigma), no 50% error;
    ///   0.25 -- 68% probability (1 sigma), no 25% error;
    ///   0.125 -- 95% probability (2 sigmas), no 25% error;
    ///   0.0625 -- 95% probability (2 sigmas), no 12.5% error:
    #[structopt(long = "eps", default_value = "0.0625", verbatim_doc_comment)]
    pub eps: f64,
    /// Per-mirror: after min measurement time elapsed, check such number of
    /// subsequently downloaded data chunks whether speed variations are less
    /// then "eps"
    #[structopt(long = "eps-checks", default_value = "40")]
    pub eps_checks: usize,
    /// Number of simultaneous speed tests
    #[structopt(long = "concurrency", default_value = "8")]
    pub concurrency: usize,
    /// Max number of jumps between countries, when finding top mirrors
    #[structopt(long = "max-jumps", default_value = "7")]
    pub max_jumps: usize,
    /// Entry country - first country (+ its neighbours) to test.
    /// You don't need to change it unless you are just curious.
    #[structopt(long = "entry-country", default_value = "US", verbatim_doc_comment)]
    pub entry_country: String,
    /// Neighbor country to test per country
    #[structopt(long = "country-neighbors-per-country", default_value = "3")]
    pub country_neighbors_per_country: usize,
    /// Number of mirrors to test per country
    #[structopt(long = "country-test-mirrors-per-country", default_value = "2")]
    pub country_test_mirrors_per_country: usize,
    /// Number of top mirrors to retest
    #[structopt(long = "top-mirrors-number-to-retest", default_value = "5")]
    pub top_mirrors_number_to_retest: usize,
    /// Test only specified protocols (can be passed multiple times)
    #[structopt(long = "protocol")]
    pub protocols: Option<Vec<String>>,
    /// Mirrors sorting strategy, one of:
    ///   score_asc, score_desc, delay_asc, delay_desc, random
    /// [default: store_asc] (lower is better)
    ///   see https://archlinux.org/mirrors/status/ for score definition
    #[structopt(long = "sort-mirrors-by", verbatim_doc_comment)]
    pub sort_mirrors_by: Option<MirrorsSortingStrategy>,
    /// Filename to save the output to in case of success
    #[structopt(long = "save", verbatim_doc_comment)]
    pub save_to_file: Option<String>,
}
