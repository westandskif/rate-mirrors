use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::arcolinux::ArcoLinuxTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

impl LogFormatter for ArcoLinuxTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}", mirror.url)
    }
}

impl FetchMirrors for ArcoLinuxTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let url =
            "https://raw.githubusercontent.com/arcolinux/arcolinux-mirrorlist/refs/heads/master/etc/pacman.d/arcolinux-mirrorlist";

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

        let urls = output
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.replace("Server = ", ""))
            .filter(|line| !line.is_empty())
            .filter_map(|line| Url::parse(&line).ok());

        let result: Vec<_> = urls
            .filter_map(|url| {
                let raw_url = url.to_string();
                const GITLAB_URL_SUFFIX: &str = "$repo/-/raw/main/$arch";
                const OTHER_URL_SUFFIX: &str = "$repo/$arch";
                if let Some(_) = raw_url.find(GITLAB_URL_SUFFIX) {
                    Url::parse(
                        (raw_url.replace(GITLAB_URL_SUFFIX, "")
                            + self.gitlab_path_to_test.as_str())
                        .as_ref(),
                    )
                    .ok()
                    .map(|url_to_test| Mirror {
                        country: None,
                        url,
                        url_to_test,
                    })
                    // https://gitlab.com/arcolinux/$repo/-/raw/main/$arch
                    // https://gitlab.com/arcolinux/arcolinux_repo_3party/-/raw/main/x86_64/arcolinux_repo_3party.files
                } else {
                    Url::parse(
                        (raw_url.replace(OTHER_URL_SUFFIX, "") + self.path_to_test.as_str())
                            .as_str(),
                    )
                    .ok()
                    .map(|url_to_test| Mirror {
                        country: None,
                        url,
                        url_to_test,
                    })
                    // https://mirror.aarnet.edu.au/pub/arcolinux/$repo/$arch
                    // https://mirror.aarnet.edu.au/pub/arcolinux/arcolinux_repo_3party/x86_64/arcolinux_repo_3party.files
                }
            })
            .collect();

        Ok(result)
    }
}
