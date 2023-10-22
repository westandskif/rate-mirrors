// https://wiki.manjaro.org/index.php/Change_to_a_Different_Download_Server

use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::manjaro::{ManjaroBranch, ManjaroTarget};
use chrono::prelude::*;
use reqwest;
use serde::{de, Deserialize, Deserializer};
use std::fmt::Display;
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
    #[serde(deserialize_with = "deserialize_last_sync")]
    last_sync: Option<i64>,
    protocols: Vec<String>,
    url: String,
}

fn deserialize_last_sync<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    if let Ok(value) = String::deserialize(deserializer) {
        let dt =
            NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M").map_err(de::Error::custom)?;
        let now = Local::now().naive_utc();
        Ok(Some((now - dt).num_seconds()))
    } else {
        Ok(None)
    }
}

impl LogFormatter for ManjaroTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}{}/$repo/$arch", &mirror.url, self.branch)
    }
}

impl FetchMirrors for ManjaroTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://mirror-manager.manjaro.org/status.json";

        let mirrors_data = Runtime::new().unwrap().block_on(async {
            Ok::<_, AppError>(
                reqwest::Client::new()
                    .get(url)
                    .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                    .send()
                    .await?
                    .json::<Vec<ManjaroMirrorData>>()
                    .await?,
            )
        })?;

        tx_progress
            .send(format!("FETCHED MIRRORS: {}", mirrors_data.len()))
            .unwrap();

        let mirrors: Vec<_> = mirrors_data
            .into_iter()
            .filter(|m| {
                m.last_sync.is_some()
                    && m.last_sync.unwrap() <= self.max_delay
                    && match self.branch {
                        ManjaroBranch::Stable => m.branches.get(0) > Some(&0),
                        ManjaroBranch::Testing => m.branches.get(1) > Some(&0),
                        ManjaroBranch::Unstable => m.branches.get(2) > Some(&0),
                        ManjaroBranch::ARMStable => m.branches.get(3) > Some(&0),
                        ManjaroBranch::ARMTesting => m.branches.get(4) > Some(&0),
                        ManjaroBranch::ARMUnstable => m.branches.get(5) > Some(&0),
                    }
            })
            .filter_map(|m| {
                let urls: Vec<_> = m
                    .protocols
                    .iter()
                    .filter_map(|p| Url::parse(&format!("{}://{}", p, &m.url)).ok())
                    .collect();

                Some((m, config.get_preferred_url(&urls)?.to_owned()))
            })
            .filter_map(|(m, url)| {
                let branch = format!("{}/", self.branch);
                url.join(&branch)
                    .and_then(|u| u.join(&self.path_to_test))
                    .map(|url_to_test| Mirror {
                        country: Country::from_str(&m.country),
                        url,
                        url_to_test,
                    })
                    .ok()
            })
            .collect();

        Ok(mirrors)
    }
}
