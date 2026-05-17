use crate::config::{AppError, FetchMirrors, LogFormatter, fetch_text_or_file};
use crate::mirror::Mirror;
use crate::target_configs::archlinuxcn::ArchCNTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

impl LogFormatter for ArchCNTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}{}", mirror.url, arch)
    }
}

impl FetchMirrors for ArchCNTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let output = fetch_text_or_file(&self.mirror_list_file, self.fetch_mirrors_timeout)?;

        let urls = output
            .lines()
            .filter_map(|line| {
                if line.starts_with("# Server = ") {
                    Some(line.replace("# Server = ", ""))
                } else if line.starts_with("Server = ") {
                    Some(line.replace("Server = ", ""))
                } else {
                    None
                }
            })
            .filter_map(|line| Url::parse(&line.replace("$arch", "")).ok());
        let result: Vec<_> = urls
            .map(|url| {
                let url_to_test = url
                    .join(&self.path_to_test)
                    .expect("failed to join path_to_test");
                Mirror {
                    country: None,
                    url,
                    url_to_test,
                }
            })
            .collect();

        Ok(result)
    }
}
