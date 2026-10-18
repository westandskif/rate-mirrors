use crate::config::parse_positive_usize;
use clap::Args;

/// Mirror-list fetch options, shared by every target that fetches a mirror list.
#[derive(Debug, Clone, Args)]
pub struct FetchArgs {
    /// Total time in milliseconds for fetching the mirror list, across all retry attempts
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_TIMEOUT",
        long,
        default_value = "30000"
    )]
    pub fetch_mirrors_timeout: u64,

    /// Maximum number of attempts for fetching the mirror list
    #[arg(
        env = "RATE_MIRRORS_FETCH_MIRRORS_ATTEMPTS",
        long,
        default_value = "3",
        value_parser = parse_positive_usize
    )]
    pub fetch_mirrors_attempts: usize,
}
