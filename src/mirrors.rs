// extern crate reqwest;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::config::{Config, MirrorsSortingStrategy};
use crate::countries::Country;
use crate::speed_test::{SpeedTestError, SpeedTestResult};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::sync::mpsc::Sender;
use tokio;
use tokio::sync::Semaphore;

#[derive(Deserialize, Debug, Clone)]
pub struct MirrorData {
    protocol: String,
    pub url: String,
    score: Option<f64>,
    delay: Option<u64>,
    active: bool,
    pub country_code: String,
    completion_pct: Option<f64>,
}
impl MirrorData {
    pub async fn test_speed(
        self,
        path_to_test: String,
        timeout: Duration,
        min_duration: Duration,
        min_bytes: usize,
        eps: f64,
        eps_checks: usize,
        semaphore: Arc<Semaphore>,
        tx_progress: Option<Sender<String>>,
    ) -> Result<SpeedTestResult<MirrorData>, SpeedTestError> {
        let mut bytes_downloaded: usize = 0;

        let _permit = semaphore.acquire().await;

        let client = reqwest::Client::new();
        let url_for_speed_test = {
            let mut url = self.url.clone();
            url.push_str(path_to_test.as_ref());
            url
        };
        let started_connecting = SystemTime::now();
        let mut response = client
            .get(&url_for_speed_test)
            .timeout(timeout)
            .send()
            .await?;
        let connection_time = started_connecting.elapsed().unwrap();
        let started_ts = SystemTime::now();
        let mut prev_ts = started_ts;
        let mut speeds: Vec<f64> = Vec::with_capacity(eps_checks);
        let mut index = 0;
        let eps_checks_f64 = eps_checks as f64;
        let mut filling_up = true;
        while let Ok(Some(chunk)) = response.chunk().await {
            let chunk_size = chunk.len();
            bytes_downloaded += chunk_size;

            let now = SystemTime::now();
            let chunk_speed =
                chunk_size as f64 / now.duration_since(prev_ts).unwrap().as_secs_f64();
            prev_ts = now;

            if filling_up {
                speeds.push(chunk_speed);
                index = (index + 1) % eps_checks;
                if index == 0 {
                    filling_up = false;
                }
            } else {
                speeds[index] = chunk_speed;
                index = (index + 1) % eps_checks;
            }
            if bytes_downloaded >= min_bytes
                && now.duration_since(started_ts).unwrap() > min_duration
                && speeds.len() == eps_checks
            {
                let mean = speeds.iter().sum::<f64>() / eps_checks_f64;
                let variance = speeds
                    .iter()
                    .map(|speed| {
                        let diff = mean - *speed;
                        diff * diff
                    })
                    .sum::<f64>()
                    / eps_checks_f64;
                let std_deviation = variance.sqrt();

                if std_deviation / mean <= eps {
                    break;
                }
            }
        }
        drop(_permit);

        if bytes_downloaded < min_bytes {
            return Err(SpeedTestError::TooFewBytesDownloadedError);
        }

        let country_code = self.country_code.clone();
        let speed_test_result = SpeedTestResult::new(
            self,
            bytes_downloaded,
            prev_ts
                .duration_since(started_ts)
                .unwrap_or_else(|_| Duration::from_millis(0)),
            connection_time,
        );
        if let Some(sender) = tx_progress.as_ref() {
            sender
                .send(format!("[{}] {}", country_code, &speed_test_result))
                .unwrap();
        }
        Ok(speed_test_result)
    }
}
#[derive(Deserialize, Debug)]
struct MirrorsData {
    urls: Vec<MirrorData>,
}

pub fn fetch_mirrors(config: Arc<Config>) -> HashMap<&'static Country, Vec<MirrorData>> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();
    let response = runtime
        .block_on(
            reqwest::Client::new()
                .get("https://www.archlinux.org/mirrors/status/json/")
                .timeout(Duration::from_millis(config.fetch_mirrors_timeout))
                .send(),
        )
        .unwrap();

    let mirrors_data = runtime.block_on(response.json::<MirrorsData>()).unwrap();
    let allowed_protocols: Vec<String> = match config.protocols.as_ref() {
        Some(protocols) => protocols.to_owned(),
        None => vec![String::from("https"), String::from("http")],
    };
    let mut mirrors: Vec<MirrorData> = mirrors_data
        .urls
        .into_iter()
        .filter(|mirror| {
            mirror
                .completion_pct
                .filter(|&pct| pct >= config.completion)
                .is_some()
                && mirror
                    .delay
                    .filter(|&delay| delay <= config.max_delay)
                    .is_some()
                && allowed_protocols.contains(&mirror.protocol)
                && mirror.country_code.len() > 0
        })
        .collect();
    match &config.sort_mirrors_by {
        Some(MirrorsSortingStrategy::Random) => {
            let mut rng = thread_rng();
            mirrors.shuffle(&mut rng);
        }
        Some(MirrorsSortingStrategy::DelayDesc) => {
            mirrors.sort_unstable_by(|a, b| b.delay.partial_cmp(&a.delay).unwrap());
        }
        Some(MirrorsSortingStrategy::DelayAsc) => {
            mirrors.sort_unstable_by(|a, b| a.delay.partial_cmp(&b.delay).unwrap());
        }
        Some(MirrorsSortingStrategy::ScoreDesc) => {
            mirrors.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        }
        Some(MirrorsSortingStrategy::ScoreAsc) | _ => {
            mirrors.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        }
    };
    let mut result: HashMap<&Country, Vec<MirrorData>> = HashMap::new();
    for mirror in mirrors.into_iter() {
        let mirrors = result
            .entry(Country::from_str(mirror.country_code.as_str()).unwrap())
            .or_insert_with(|| Vec::new());
        mirrors.push(mirror);
    }

    result
}
