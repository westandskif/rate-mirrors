use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::artix::ArtixTarget;
use reqwest;
use std::fmt::Display;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

impl LogFormatter for ArtixTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}$repo/os/$arch", mirror.url)
    }
}

impl FetchMirrors for ArtixTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        _tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://packages.artixlinux.org/mirrorlist/all/";

        let output = Runtime::new().unwrap().block_on(async {
            Ok::<_, AppError>(
                reqwest::Client::new()
                    .get(url)
                    .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                    .send()
                    .await?
                    .text_with_charset("utf-8")
                    .await?,
            )
        })?;

        let mut current_country = None;
        let mut mirrors = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("##") {
                let country_name = trimmed
                    .trim_start_matches('#')
                    .trim_start_matches('#')
                    .trim_start();
                current_country = Country::from_str(country_name);
                continue;
            }

            let uncommented = trimmed.trim_start_matches('#').trim_start();
            if !uncommented.starts_with("Server = ") {
                continue;
            }

            let cleaned = uncommented
                .trim_start_matches("Server = ")
                .replace("$repo/os/$arch", "");

            if cleaned.is_empty() {
                continue;
            }

            if let Ok(url) = Url::parse(&cleaned) {
                if let Ok(protocol) = url.scheme().parse() {
                    if config.is_protocol_allowed(&protocol) {
                        mirrors.push(Mirror {
                            country: current_country,
                            url_to_test: url
                                .join(&self.path_to_test)
                                .expect("failed to join path_to_test"),
                            url,
                        });
                    }
                }
            }
        }

        Ok(mirrors)
    }
}
