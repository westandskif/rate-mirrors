use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::chaotic::ChaoticTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

impl LogFormatter for ChaoticTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}$repo/{}", mirror.url, arch)
    }
}

impl FetchMirrors for ChaoticTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let url = "https://gitlab.com/chaotic-aur/pkgbuilds/-/raw/main/chaotic-mirrorlist/mirrorlist";

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

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
            .filter_map(|line| Url::parse(&line.replace("$repo/$arch", "")).ok());
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
