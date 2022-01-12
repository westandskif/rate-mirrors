use crate::config::{AppError, Config, FetchMirrors};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::archlinux::{ArchMirrorsSortingStrategy, ArchTarget};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use reqwest;
use serde::Deserialize;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use url::Url;

// Server = {}$repo/os/$arch
// "community/os/x86_64/community.files"

#[derive(Deserialize, Debug, Clone)]
pub struct ArchMirror {
    protocol: String,
    url: String,
    score: Option<f64>,
    delay: Option<u64>,
    // active: bool,
    country_code: String,
    completion_pct: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct ArchMirrorsData {
    urls: Vec<ArchMirror>,
}

impl FetchMirrors for ArchTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://www.archlinux.org/mirrors/status/json/";

        let mirrors_data = reqwest::blocking::Client::new()
            .get(url)
            .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
            .send()?
            .json::<ArchMirrorsData>()?;

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
                let mut rng = thread_rng();
                mirrors.shuffle(&mut rng);
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
                            output: format!("Server = {}$repo/os/$arch", &m.url),
                            url,
                            url_to_test,
                        });
                    }
                };
                None
            })
            .collect();
        tx_progress
            .send(format!("MIRRORS LEFT AFTER FILTERING: {}", result.len()))
            .unwrap();
        Ok(result)
    }
}
