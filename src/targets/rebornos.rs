use super::stdin::Mirror;
use crate::config::{Config, Protocol};
use crate::target_configs::rebornos::RebornOSTarget;
use regex::Regex;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;

pub fn fetch_rebornos_mirrors(
    config: Arc<Config>,
    target: RebornOSTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _sth = runtime.enter();
    let response = runtime
        .block_on(
            reqwest::Client::new()
                .get("https://gitlab.com/rebornos-team/rebornos-special-system-files/mirrors/reborn-mirrorlist/-/raw/master/reborn-mirrorlist")
                .timeout(Duration::from_millis(target.fetch_mirrors_timeout))
                .send(),
        )
        .unwrap();

    let mirrorlist_file_text = runtime
        .block_on(response.text_with_charset("utf-16"))
        .unwrap();
    
    // Use https://regex101.com to ensure that the regex is correct and to modify it
    let url_regex = Regex::new(r#"/(?x)     # Spaces and comments the pattern are ignored
    ^                       # Start of the line
    .*?                     # Any number of characters (lazy, minimize the number of matches)
    (?P<URL> [[:alpha:]]*:\/\/.*)   # The URL to be captured
    \s*                     # Any whitespace at the end of the URL
    $                       # End of the line
    /mgu"#, // Multiline, Global, and Unicode flags
    ).unwrap();

    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };
    let mirrors: Vec<Mirror> = url_regex
        .captures_iter(&mirrorlist_file_text)
        .map(|capture| capture["URL"])
        .filter_map(|potential_url| Mirror::line_to_mirror_info(potential_url).ok())
        .filter_map(|(url, country)| {
            Some(Mirror {
                country: country,
                output: format!("Server = {}", url.as_str().to_owned()),
                url_to_test: url
                    .join(&target.path_to_test)
                    .expect("failed to join path-to-test"),
                url: url,
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
