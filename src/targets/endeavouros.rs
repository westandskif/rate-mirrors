use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::endeavouros::EndeavourOSTarget;
use futures::future::join_all;
use reqwest;
use std::fmt::Display;
use std::fs;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio;
use tokio::sync::Semaphore;
use url::Url;

struct VersionedMirror {
    pub mirror: Mirror,
    pub update_number: Option<usize>,
}

async fn version_mirror(
    mirror: Mirror,
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
    let msg = match response_result { Ok(response) => {
        match response.text_with_charset("utf-8").await { Ok(output) => {
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
        } _ => {
            format!("FAILED TO READ STATE: {}", mirror.url)
        }}
    } _ => {
        format!("FAILED TO CONNECT: {}", mirror.url)
    }};

    tx_progress.send(msg).unwrap();

    VersionedMirror {
        mirror,
        update_number,
    }
}

fn version_mirrors(
    target: Arc<EndeavourOSTarget>,
    mirrors: Vec<Mirror>,
    tx_progress: mpsc::Sender<String>,
) -> Vec<VersionedMirror> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();

    let semaphore = Arc::new(Semaphore::new(target.version_mirror_concurrency));

    let handles = mirrors.into_iter().map(|mirror| {
        runtime.spawn(version_mirror(
            mirror,
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

impl LogFormatter for EndeavourOSTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}$repo/$arch", &mirror.url)
    }
}

impl FetchMirrors for EndeavourOSTarget {
    fn fetch_mirrors(
        &self,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let output = if let Ok(_) = Url::parse(self.mirror_list_file.as_str()) {
            fetch_text(&self.mirror_list_file, self.fetch_mirrors_timeout)?
        } else {
            fs::read_to_string(self.mirror_list_file.as_str())
                .map_err(|e| AppError::RequestError(format!("failed to read mirror-list-file: {}", e)))?
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
                let url_to_test = url
                    .join(&self.path_to_test)
                    .expect("failed to join path_to_test");
                mirrors.push(Mirror {
                    country: current_country,
                    url,
                    url_to_test,
                });
            }
        }

        let versioned_mirrors = version_mirrors(
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

        Ok(mirrors)
    }
}
