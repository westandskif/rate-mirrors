use super::stdin::Mirror;
use crate::config::{Config, Protocol};
use crate::target_configs::cachyos::CachyOSTarget;
use reqwest;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio;
use url::Url;

pub fn fetch_cachyos_mirrors(
    config: Arc<Config>,
    target: CachyOSTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();
    let url = "https://raw.githubusercontent.com/CachyOS/CachyOS-PKGBUILDS/master/cachyos-mirrorlist/cachyos-mirrorlist";
    let response = runtime
        .block_on(
            reqwest::Client::new()
                .get(url)
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

    let output = runtime
        .block_on(response.text_with_charset("utf-8"))
        .expect(format!("failed to fetch mirrors from {}", url).as_str());

    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };

    let urls: Vec<Url> = output
        .lines()
        .filter(|line| !line.starts_with("#"))
        .map(|line| line.replace("Server = ", "").replace("$arch/$repo", ""))
        .filter(|line| line.len() > 0)
        .filter_map(|line| Url::from_str(&line).ok())
        .filter(|url| match Protocol::from_str(url.scheme()) {
            Ok(protocol) => allowed_protocols.contains(&protocol),
            Err(_) => false,
        })
        .collect();

    let arch: String = match target.arch.as_str() {
        "auto" => String::from("$arch"),
        _ => String::from(target.arch.as_str()),
    };
    let result: Vec<Mirror> = urls
        .into_iter()
        .filter_map(|url| {
            let url_to_test = url
                .join(&target.path_to_test)
                .expect("failed to join path_to_test");
            Some(Mirror {
                country: None,
                output: format!("Server = {}{}/$repo", &url.as_str(), &arch.as_str()),
                url,
                url_to_test,
            })
        })
        .collect();
    tx_progress
        .send(format!("MIRRORS LEFT AFTER FILTERING: {}", result.len()))
        .unwrap();
    result
}
