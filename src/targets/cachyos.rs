use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::cachyos::CachyOSTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

impl LogFormatter for CachyOSTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}{}/$repo", mirror.url, arch)
    }
}

impl FetchMirrors for CachyOSTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let url = "https://raw.githubusercontent.com/CachyOS/CachyOS-PKGBUILDS/master/cachyos-mirrorlist/cachyos-mirrorlist";

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

        let urls = output
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.replace("Server = ", "").replace("$arch/$repo", ""))
            .filter(|line| !line.is_empty())
            .filter_map(|line| Url::parse(&line).ok());

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
