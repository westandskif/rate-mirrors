use super::stdin::Mirror;
use crate::config::{Config, Protocol};
use crate::target_configs::rebornos::RebornOSTarget;
use linkify::{LinkFinder, LinkKind};
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use url::Url;

fn text_from_url(url: Url, timeout: u64) -> String {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();
    let response = runtime
        .block_on(
            reqwest::Client::new()
                .get(url.as_str())
                .timeout(Duration::from_millis(timeout))
                .send(),
        )
        .unwrap();

    return runtime
        .block_on(response.text_with_charset("utf-16"))
        .unwrap();
}

pub fn fetch_rebornos_mirrors(
    config: Arc<Config>,
    target: RebornOSTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };

    let mirrorlist_file_text = text_from_url(
        Url::from_str(
            "https://gitlab.com/rebornos-team/rebornos-special-system-files/mirrors/reborn-mirrorlist/-/raw/master/reborn-mirrorlist"
        ).unwrap(),
        target.fetch_mirrors_timeout
    );

    let mut link_finder = LinkFinder::new();
    link_finder.kinds(&[LinkKind::Url]);
    let url_iter = link_finder
        .links(&mirrorlist_file_text)
        .filter_map(|url| Url::from_str(url.as_str()).ok());

    let mirrors: Vec<Mirror> = url_iter
        .filter_map(|url| {
            Some(Mirror {
                country: None,
                output: format!("Server = {}", url.as_str().to_owned()),
                url_to_test: url
                    .join(&target.path_to_test)
                    .expect("failed to join path-to-test"),
                url,
            })
        })
        .filter(|mirror| {
            allowed_protocols.contains(&Protocol::from_str(mirror.url.scheme()).unwrap())
        })
        .collect();
    tx_progress
        .send(format!("FETCHED {} MIRRORS FROM REBORNOS", mirrors.len()))
        .unwrap();

    mirrors
}
