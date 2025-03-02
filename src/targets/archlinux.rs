use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::archlinux::{ArchMirrorsSortingStrategy, ArchTarget};
use rand::prelude::SliceRandom;
use rand::rng;
use reqwest;
use serde::Deserialize;
use std::fmt::Display;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct ArchMirror {
    protocol: String,
    url: String,
    score: Option<f64>,
    delay: Option<i64>,
    // active: bool,
    country_code: String,
    completion_pct: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct ArchMirrorsData {
    urls: Vec<ArchMirror>,
}

impl LogFormatter for ArchTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}$repo/os/$arch", &mirror.url)
    }
}

impl FetchMirrors for ArchTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = if self.fetch_first_tier_only {
            "https://archlinux.org/mirrors/status/tier/1/json/"
        } else {
            "https://archlinux.org/mirrors/status/json/"
        };

        let mirrors_data = Runtime::new().unwrap().block_on(async {
            Ok::<_, AppError>(
                reqwest::Client::new()
                    .get(url)
                    .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                    .send()
                    .await?
                    .json::<ArchMirrorsData>()
                    .await?,
            )
        })?;

        tx_progress
            .send(format!("FETCHED MIRRORS: {}", mirrors_data.urls.len()))
            .unwrap();

        let mut mirrors: Vec<_> = mirrors_data
            .urls
            .into_iter()
            .filter(|mirror| {
                if let Some(completion_pct) = mirror.completion_pct {
                    if let Some(delay) = mirror.delay {
                        if let Ok(protocol) = mirror.protocol.parse() {
                            return completion_pct >= self.completion
                                && delay <= self.max_delay
                                && config.is_protocol_allowed(&protocol)
                                && !mirror.country_code.is_empty();
                        }
                    }
                }
                false
            })
            .collect();

        match &self.sort_mirrors_by {
            ArchMirrorsSortingStrategy::Random => {
                let mut _rng = rng();
                mirrors.shuffle(&mut _rng);
            }
            ArchMirrorsSortingStrategy::DelayDesc => {
                mirrors.sort_unstable_by(|a, b| b.delay.partial_cmp(&a.delay).unwrap());
            }
            ArchMirrorsSortingStrategy::DelayAsc => {
                mirrors.sort_unstable_by(|a, b| a.delay.partial_cmp(&b.delay).unwrap());
            }
            ArchMirrorsSortingStrategy::ScoreDesc => {
                mirrors.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            }
            ArchMirrorsSortingStrategy::ScoreAsc => {
                mirrors.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            }
        };

        let result: Vec<_> = mirrors
            .into_iter()
            .filter_map(|m| {
                if let Ok(url) = Url::parse(&m.url) {
                    if let Ok(url_to_test) = url.join(&self.path_to_test) {
                        return Some(Mirror {
                            country: Country::from_str(&m.country_code),
                            url,
                            url_to_test,
                        });
                    }
                };
                None
            })
            .collect();

        Ok(result)
    }
}
