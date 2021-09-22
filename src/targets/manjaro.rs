// https://wiki.manjaro.org/index.php/Change_to_a_Different_Download_Server

use super::stdin::Mirror;
use crate::config::{Config, Protocol};
use crate::countries::Country;
use crate::target_configs::manjaro::{ManjaroBranch, ManjaroTarget};
use reqwest;
use serde::Deserialize;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;
// [
//   {
//     "branches": [1, 1, 0],
//     "country": "Australia",
//     "last_sync": "02:13",
//     "protocols": ["https"],
//     "url": "https://manjaro.lucassymons.net/"
//   },

#[derive(Deserialize, Debug, Clone)]
pub struct ManjaroMirrorData {
    branches: Vec<i8>,
    country: String,
    last_sync: serde_json::Value,
    protocols: Vec<String>,
    url: String,
}

pub struct PreparedManjaroMirrorData {
    branches: (bool, bool, bool),
    country: Option<&'static Country>,
    delay: usize,
    // protocols: Vec<Protocol>,
    url: Url,
}
impl ManjaroMirrorData {
    pub fn to_prepared(
        &self,
        allowed_protocols: &[Protocol],
    ) -> Result<Option<PreparedManjaroMirrorData>, &str> {
        if self.branches.len() != 3 {
            return Err("unknown branches format");
        }
        let protocols: Vec<Protocol> = self
            .protocols
            .iter()
            .filter_map(|protocol| protocol.parse::<Protocol>().ok())
            .collect();
        let https_protocol = Protocol::Https;
        let http_protocol = Protocol::Http;
        let scheme;
        if allowed_protocols.contains(&https_protocol) && protocols.contains(&https_protocol) {
            scheme = "https";
        } else if allowed_protocols.contains(&http_protocol) && protocols.contains(&http_protocol) {
            scheme = "http";
        } else {
            return Ok(None);
        }
        let url: Url = match Url::parse(self.url.as_str()) {
            Ok(mut url) => {
                url.set_scheme(scheme).unwrap();
                url
            }
            Err(_) => return Err("failed to parse url"),
        };
        let last_sync_numbers: Vec<usize>;
        if self.last_sync.is_string() {
            last_sync_numbers = self
                .last_sync
                .as_str()
                .unwrap()
                .split(":")
                .into_iter()
                .filter_map(|num_as_str| num_as_str.parse::<usize>().ok())
                .collect();
        } else {
            last_sync_numbers = vec![];
        }
        let delay: usize = match last_sync_numbers.len() {
            2 => (last_sync_numbers.get(0).unwrap() * 60 + last_sync_numbers.get(1).unwrap()) * 60,
            _ => {
                return Err("failed to parse last_sync");
            }
        };
        Ok(Some(PreparedManjaroMirrorData {
            branches: (
                *self.branches.get(0).unwrap() > 0,
                *self.branches.get(1).unwrap() > 0,
                *self.branches.get(2).unwrap() > 0,
            ),
            country: Country::from_str(self.country.as_str()),
            delay,
            // protocols,
            url,
        }))
    }
}
pub fn fetch_manjaro_mirrors(
    config: Arc<Config>,
    target: ManjaroTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();
    let response = runtime
        .block_on(
            reqwest::Client::new()
                .get("https://repo.manjaro.org/status.json")
                .timeout(Duration::from_millis(target.fetch_mirrors_timeout as u64))
                .send(),
        )
        .expect("failed to fetch manjaro mirrors");
    let raw_response = runtime
        .block_on(response.text())
        .expect("failed to fetch manjaro mirrors");
    let mirrors_data: Vec<ManjaroMirrorData> =
        serde_json::from_str(&raw_response).expect("failed to parse manjaro mirrors");
    tx_progress
        .send(format!("FETCHED MIRRORS: {}", mirrors_data.len()))
        .unwrap();

    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };

    let mirrors: Vec<Mirror> = mirrors_data
        .into_iter()
        .filter_map(|mirror_data| mirror_data.to_prepared(allowed_protocols).ok())
        .filter_map(|mirror_data| {
            mirror_data.filter(|m| {
                m.delay <= target.max_delay
                    && match target.branch {
                        ManjaroBranch::Stable => m.branches.0,
                        ManjaroBranch::Testing => m.branches.1,
                        ManjaroBranch::Unstable => m.branches.2,
                    }
            })
        })
        .filter_map(|prepared_mirror| {
            let branch = format!("{}/", target.branch.as_str());
            let prepared_url = match prepared_mirror.url.join(&branch) {
                Ok(url) => url,
                Err(_) => return None,
            };
            let url_to_test = match prepared_url.join(&target.path_to_test) {
                Ok(url) => url,
                Err(_) => return None,
            };
            Some(Mirror {
                country: prepared_mirror.country,
                output: format!("Server = {}$repo/$arch", &prepared_url),
                url: prepared_url,
                url_to_test,
            })
        })
        .collect();
    tx_progress
        .send(format!("MIRRORS LEFT AFTER FILTERING: {}", mirrors.len()))
        .unwrap();
    mirrors
}
