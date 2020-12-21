use std::fmt::Debug;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "hey text")]
pub struct Config {
    /// Completion percentage [0, 1]
    #[structopt(long = "completion", default_value = "1")]
    pub completion: f64,
    /// Max acceptable delay in seconds
    #[structopt(long = "max-delay", default_value = "86400")]
    pub max_delay: u64,
    /// Fetch mirrors timeout in milliseconds
    #[structopt(long = "fetch-mirrors-timeout", default_value = "15000")]
    pub fetch_mirrors_timeout: u64,
    /// Per-mirror timeout in milliseconds
    #[structopt(long = "per-mirror-timeout", default_value = "1500")]
    pub per_mirror_timeout: u64,
    /// Per-mirror min measurement time in milliseconds
    #[structopt(long = "min-per-mirror", default_value = "300")]
    pub min_per_mirror: u64,
    /// Mirror path to test speed
    #[structopt(
        long = "mirror-path",
        default_value = "community/os/x86_64/community.files"
    )]
    pub path_to_test: String,
    /// Minimum bytes to be downloded
    #[structopt(long = "min-bytes-per-mirror", default_value = "70000")]
    pub min_bytes_per_mirror: usize,
    /// Per-mirror: sigma to mean speed ratio
    ///   1.0 -- 68% probability (1 sigma), no 100% error;
    ///   0.5 -- 68% probability (1 sigma), no 50% error;
    ///   0.25 -- 68% probability (1 sigma), no 25% error;
    ///   0.125 -- 95% probability (2 sigmas), no 25% error;
    ///   0.0625 -- 95% probability (2 sigmas), no 12.5% error:
    #[structopt(long = "eps", default_value = "0.0625")]
    pub eps: f64,
    /// Per-mirror: after min measurement time elapsed, check such number of
    /// subsequently downloaded chunks whether speed variations are less then
    /// "eps"
    #[structopt(long = "eps-checks", default_value = "40")]
    pub eps_checks: usize,
    /// Number of simultaneous checks
    #[structopt(long = "concurrency", default_value = "8")]
    pub concurrency: usize,
    /// Max number of jumps between countries
    #[structopt(long = "max-jumps", default_value = "7")]
    pub max_jumps: usize,
    /// Entry country
    #[structopt(long = "entry-country", default_value = "US")]
    pub entry_country: String,
    /// Country neighbors to test per country
    #[structopt(long = "country-neighbors-per-country", default_value = "3")]
    pub country_neighbors_per_country: usize,
    /// Number of mirrors to test per country
    #[structopt(long = "country-test-mirrors-per-country", default_value = "2")]
    pub country_test_mirrors_per_country: usize,
    /// Number of top mirrors to retest
    #[structopt(long = "top-mirrors-number-to-retest", default_value = "5")]
    pub top_mirrors_number_to_retest: usize,
}
