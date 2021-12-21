use crate::target_configs::archlinux::ArchTarget;
use crate::target_configs::artix::ArtixTarget;
use crate::target_configs::manjaro::ManjaroTarget;
use crate::target_configs::rebornos::RebornOSTarget;
use crate::target_configs::stdin::StdinTarget;
use std::fmt::Debug;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, PartialEq, Clone)]
pub enum Protocol {
    Http,
    Https,
}
impl FromStr for Protocol {
    type Err = &'static str;
    fn from_str(protocol: &str) -> Result<Self, Self::Err> {
        match protocol {
            "http" => Ok(Protocol::Http),
            "https" => Ok(Protocol::Https),
            _ => Err("could not parse protocol"),
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum Target {
    /// accepts lines of urls OR lines with tab-separated urls and countries
    Stdin(StdinTarget),
    /// fetch & test archlinux mirrors
    Arch(ArchTarget),
    /// fetch & test manjaro mirrors
    Manjaro(ManjaroTarget),
    /// fetch & test rebornos mirrors
    #[structopt(name = "rebornos")]
    RebornOS(RebornOSTarget),
    /// fetch & test artix mirrors
    Artix(ArtixTarget),
}

#[derive(Debug, StructOpt)]
#[structopt(name = "rate-mirrors config")]
/// Usually default options should work
pub struct Config {
    /// Per-mirror speed test timeout in milliseconds
    #[structopt(subcommand)]
    pub target: Target,

    /// Test only specified protocols (can be passed multiple times)
    #[structopt(long = "protocol", name = "protocol", number_of_values = 1)]
    pub protocols: Vec<Protocol>,

    /// Per-mirror speed test timeout in milliseconds
    #[structopt(long = "per-mirror-timeout", default_value = "1500")]
    pub per_mirror_timeout: u64,

    /// Minimum downloading time, required to measure mirror speed,
    /// in milliseconds
    #[structopt(long = "min-per-mirror", default_value = "300")]
    pub min_per_mirror: u64,

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

    /// Filename to save the output to in case of success
    #[structopt(long = "save", verbatim_doc_comment)]
    pub save_to_file: Option<String>,

    /// allow running by root
    #[structopt(long = "allow-root")]
    pub allow_root: bool,
}
