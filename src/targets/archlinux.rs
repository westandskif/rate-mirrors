use super::stdin::Mirror;
use crate::config::{Config, Protocol};
use crate::countries::Country;
use crate::target_configs::archlinux::{ArchMirrorsSortingStrategy, ArchTarget};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use reqwest;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio;
use url::Url;

// Server = {}$repo/os/$arch
// "community/os/x86_64/community.files"

#[derive(Deserialize, Debug, Clone)]
pub struct ArchMirrorData {
    protocol: String,
    pub url: String,
    score: Option<f64>,
    delay: Option<u64>,
    active: bool,
    pub country_code: String,
    completion_pct: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct ArchMirrorsData {
    urls: Vec<ArchMirrorData>,
}

pub fn fetch_arch_mirrors(
    config: Arc<Config>,
    target: ArchTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();
    let response = runtime
        .block_on(
            reqwest::Client::new()
                .get("https://www.archlinux.org/mirrors/status/json/")
                .timeout(Duration::from_millis(target.fetch_mirrors_timeout))
                .send(),
        )
        .unwrap();

    let mirrors_data = runtime
        .block_on(response.json::<ArchMirrorsData>())
        .unwrap();
    tx_progress
        .send(format!("FETCHED MIRRORS: {}", mirrors_data.urls.len()))
        .unwrap();
    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };
    let mut mirrors: Vec<ArchMirrorData> = mirrors_data
        .urls
        .into_iter()
        .filter(|mirror| {
            mirror
                .completion_pct
                .filter(|&pct| pct >= target.completion)
                .is_some()
                && mirror
                    .delay
                    .filter(|&delay| delay <= target.max_delay)
                    .is_some()
                && match Protocol::from_str(mirror.protocol.as_str()) {
                    Ok(protocol) => allowed_protocols.contains(&protocol),
                    Err(_) => false,
                }
                && mirror.country_code.len() > 0
        })
        .collect();
    match &target.sort_mirrors_by {
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
    let result: Vec<Mirror> = mirrors
        .into_iter()
        .filter_map(|m| {
            let url = match Url::parse(m.url.as_ref()) {
                Ok(url) => url,
                Err(_) => return None,
            };
            
            let url_to_test = match url.join(&target.path_to_test) {
                Ok(url) => url,
                Err(_) => return None,
            };
            Some(Mirror {
                country: Country::from_str(&m.country_code),
                output: format!("Server = {}$repo/os/$arch", &m.url),
                url: url,
                url_to_test: url_to_test,
            })
        })
        .collect();
    tx_progress
        .send(format!("MIRRORS LEFT AFTER FILTERING: {}", result.len()))
        .unwrap();
    result
}
