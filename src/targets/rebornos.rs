use crate::config::{AppError, FetchMirrors, LogFormatter, fetch_text_or_file};
use crate::mirror::Mirror;
use crate::target_configs::rebornos::RebornOSTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

impl LogFormatter for RebornOSTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}", mirror.url)
    }
}

impl FetchMirrors for RebornOSTarget {
    fn fetch_mirrors(&self, tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let output = fetch_text_or_file(&self.mirror_list_file, &self.fetch, &tx_progress)?;

        let mirrors = output
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.replace("Server = ", ""))
            .filter(|line| !line.is_empty())
            .filter_map(|line| Url::parse(&line).ok())
            .filter_map(|url| Mirror::with_test_path(url, &self.path_to_test, None))
            .collect();

        Ok(mirrors)
    }
}
