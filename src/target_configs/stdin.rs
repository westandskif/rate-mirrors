use clap::Args;
use std::fmt::Debug;

#[derive(Debug, Clone, Args)]
pub struct StdinTarget {
    /// Path to be joined to a mirror url and used for speed testing
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_TEST",
        long,
        default_value = "",
        verbatim_doc_comment
    )]
    pub path_to_test: String,

    /// Path to be joined to a mirror url before returning results
    #[arg(
        env = "RATE_MIRRORS_PATH_TO_RETURN",
        long,
        default_value = "",
        verbatim_doc_comment
    )]
    pub path_to_return: String,

    /// comment prefix to use when printing debug info
    #[arg(env = "RATE_MIRRORS_COMMENT_PREFIX", long, default_value = "# ")]
    pub comment_prefix: String,

    /// output prefix to use when printing results
    #[arg(env = "RATE_MIRRORS_OUTPUT_PREFIX", long, default_value = "")]
    pub output_prefix: String,

    /// input separator to use when parsing mirrors list
    #[arg(env = "RATE_MIRRORS_SEPARATOR", long, default_value = "\t")]
    pub input_separator: String,
}
