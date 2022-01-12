use crate::config::{AppError, Config, FetchMirrors};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::endeavouros::EndeavourOSTarget;
use futures::future::join_all;
use reqwest;
use std::fs;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;
use url::Url;

struct VersionedMirror {
    pub mirror: Mirror,
    pub update_number: Option<usize>,
}

async fn version_mirror(
    mirror: Mirror,
    _config: Arc<Config>,
    target: Arc<EndeavourOSTarget>,
    semaphore: Arc<Semaphore>,
    tx_progress: mpsc::Sender<String>,
) -> VersionedMirror {
    let _permit = semaphore.acquire().await;

    let client = reqwest::Client::new();
    let response_result = client
        .get(mirror.url.join("state").unwrap())
        .timeout(Duration::from_millis(target.version_mirror_timeout))
        .send()
        .await;

    let mut update_number = None;
    let msg = if let Ok(response) = response_result {
        if let Ok(output) = response.text_with_charset("utf-8").await {
            if let Some(line) = output.lines().next() {
                if let Ok(number) = line.parse::<usize>() {
                    update_number = Some(number);
                    format!("FETCHED MIRROR VERSION {}: {}", number, mirror.url)
                } else {
                    format!("FAILED TO READ MIRROR UPDATE NUMBER: {}", mirror.url)
                }
            } else {
                format!("EMPTY MIRROR STATE: {}", mirror.url)
            }
        } else {
            format!("FAILED TO READ STATE: {}", mirror.url)
        }
    } else {
        format!("FAILED TO CONNECT: {}", mirror.url)
    };

    tx_progress.send(msg).unwrap();

    VersionedMirror {
        mirror,
        update_number,
    }
}

fn version_mirrors(
    config: Arc<Config>,
    target: Arc<EndeavourOSTarget>,
    mirrors: Vec<Mirror>,
    tx_progress: mpsc::Sender<String>,
) -> Vec<VersionedMirror> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();

    let semaphore = Arc::new(Semaphore::new(target.version_mirrors_concurrency));

    let handles = mirrors.into_iter().map(|mirror| {
        runtime.spawn(version_mirror(
            mirror,
            Arc::clone(&config),
            Arc::clone(&target),
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        ))
    });

    runtime
        .block_on(join_all(handles))
        .into_iter()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>()
}

impl FetchMirrors for EndeavourOSTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let output = if let Ok(url) = Url::parse(self.mirror_list_file.as_str()) {
            Runtime::new().unwrap().block_on(async {
                Ok::<_, AppError>(
                    reqwest::Client::new()
                        .get(url)
                        .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                        .send()
                        .await?
                        .text_with_charset("utf-8")
                        .await?,
                )
            })?
        } else {
            fs::read_to_string(self.mirror_list_file.as_str())
                .expect("failed to read from mirror-list-file")
        };

        let mut current_country = None;
        let mut mirrors: Vec<Mirror> = Vec::new();

        for line in output.lines() {
            if line.starts_with("## ") {
                current_country = Country::from_str(line.replace("## ", "").as_str());
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            let line = line.replace("Server = ", "").replace("$repo/$arch", "");
            if line.is_empty() {
                continue;
            }
            if let Ok(url) = Url::from_str(&line) {
                if let Ok(protocol) = url.scheme().parse() {
                    if config.is_protocol_allowed(&protocol) {
                        let url_to_test = url
                            .join(&self.path_to_test)
                            .expect("failed to join path_to_test");
                        mirrors.push(Mirror {
                            country: current_country,
                            output: format!("Server = {}$repo/$arch", &url.as_str()),
                            url,
                            url_to_test,
                        });
                    }
                }
            }
        }

        let versioned_mirrors = version_mirrors(
            Arc::clone(&config),
            Arc::new(self.clone()),
            mirrors,
            mpsc::Sender::clone(&tx_progress),
        );

        let max_version = versioned_mirrors
            .iter()
            .filter_map(|m| m.update_number)
            .max();

        let mirrors = if let Some(version) = max_version {
            tx_progress
                .send(format!("TAKING MIRRORS WITH LATEST VERSION: {}", version))
                .unwrap();
            versioned_mirrors
                .into_iter()
                .filter_map(|m| {
                    if m.update_number == max_version {
                        Some(m.mirror)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        tx_progress
            .send(format!("MIRRORS LEFT AFTER FILTERING: {}", mirrors.len()))
            .unwrap();

        Ok(mirrors)
    }
}
