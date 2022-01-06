use super::stdin::Mirror;
use crate::config::{Config, Protocol};
use crate::countries::Country;
use crate::target_configs::endeavouros::EndeavourOSTarget;
use futures::future::join_all;
use reqwest;
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
    _config: Arc<Config>,
    target: Arc<EndeavourOSTarget>,
    semaphore: Arc<Semaphore>,
    tx_progress: mpsc::Sender<String>,
) -> VersionedMirror {
    let _permit = semaphore.acquire().await;

    let client = reqwest::Client::new();
    let response_result = client
        .get(mirror.url.join("state").unwrap().as_str())
        .timeout(Duration::from_millis(target.version_mirror_timeout))
        .send()
        .await;
    let mut update_number = None;
    let msg;
    if let Ok(response) = response_result {
        if let Ok(output) = response.text_with_charset("utf-8").await {
            let lines: Vec<&str> = output.lines().take(1).collect();
            if lines.len() != 0 {
                if let Ok(number) = lines[0].parse::<usize>() {
                    update_number = Some(number);
                    msg = format!("FETCHED MIRROR VERSION {}: {}", number, mirror.url.as_str());
                } else {
                    msg = format!(
                        "FAILED TO READ MIRROR UPDATE NUMBER: {}",
                        mirror.url.as_str()
                    );
                }
            } else {
                msg = format!("EMPTY MIRROR STATE: {}", mirror.url.as_str())
            }
        } else {
            msg = format!("FAILED TO READ STATE: {}", mirror.url.as_str());
        }
    } else {
        msg = format!("FAILED TO CONNECT: {}", mirror.url.as_str());
    }

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

    let mut handles = Vec::with_capacity(mirrors.len());
    for mirror in mirrors.into_iter() {
        handles.push(runtime.spawn(version_mirror(
            mirror,
            Arc::clone(&config),
            Arc::clone(&target),
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        )));
    }
    let versioned_mirrors: Vec<VersionedMirror> = runtime
        .block_on(join_all(handles))
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();
    versioned_mirrors
}

pub fn fetch_mirrors(
    config: Arc<Config>,
    target: EndeavourOSTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let output;
    if let Ok(url) = Url::from_str(target.mirror_list_file.as_str()) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _sth = runtime.enter();
        let response = runtime
            .block_on(
                reqwest::Client::new()
                    .get(url.as_str())
                    .timeout(Duration::from_millis(target.fetch_mirrors_timeout))
                    .send(),
            )
            .expect(
                format!(
                    "failed to connect to {}, consider increasing fetch-mirrors-timeout",
                    url
                )
                .as_str(),
            );

        output = runtime
            .block_on(response.text_with_charset("utf-8"))
            .expect(format!("failed to fetch mirrors from {}", url).as_str());
    } else {
        output = fs::read_to_string(target.mirror_list_file.as_str())
            .expect("failed to read from mirror-list-file");
    }

    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };

    let mut current_country = None;
    let mut mirrors: Vec<Mirror> = Vec::new();
    for line in output.lines() {
        if line.starts_with("## ") {
            current_country = Country::from_str(line.replace("## ", "").as_str());
            continue;
        }
        if line.starts_with("#") {
            continue;
        }
        let line = line.replace("Server = ", "").replace("$repo/$arch", "");
        if line.len() == 0 {
            continue;
        }
        if let Ok(url) = Url::from_str(&line) {
            if let Ok(protocol) = Protocol::from_str(url.scheme()) {
                if allowed_protocols.contains(&protocol) {
                    let url_to_test = url
                        .join(&target.path_to_test)
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
        Arc::new(target),
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
    mirrors
}
