use crate::config::{AppError, FetchMirrors, LogFormatter, fetch_text_or_file};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::cachyos::CachyOSTarget;
use serde::Deserialize;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

/// A single entry of the CachyOS dashboard mirrors API.
#[derive(Deserialize, Debug, Clone)]
struct CachyMirror {
    country_code: String,
    url: String,
    delay_seconds: Option<i64>,
}

#[derive(Deserialize, Debug)]
struct CachyMirrorsData {
    mirrors: Vec<CachyMirror>,
}

/// Resolve a raw upstream country code to a `Country`.
/// `GLOBAL` (case-insensitive, worldwide CDN) and unknown codes map to `None`.
fn resolve_country(code: &str) -> Option<&'static Country> {
    if code.eq_ignore_ascii_case("GLOBAL") {
        None
    } else {
        Country::from_str(code)
    }
}

/// Extract the `code=XX` token from a CachyOS metadata comment.
/// Returns `None` when the line carries no `code=` token.
fn country_code_from_line(line: &str) -> Option<&str> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix("code="))
}

impl LogFormatter for CachyOSTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}{}/$repo", mirror.url, arch)
    }
}

impl CachyOSTarget {
    /// Parse the dashboard JSON API response.
    fn parse_api_json(&self, raw: &str) -> Option<Vec<Mirror>> {
        let data: CachyMirrorsData = serde_json::from_str(raw).ok()?;

        let result = data
            .mirrors
            .into_iter()
            // Apply the sync-delay threshold only when the source reports a
            // delay.
            .filter(|m| m.delay_seconds.is_none_or(|delay| delay <= self.max_delay))
            .filter_map(|m| {
                let url = Url::parse(&m.url).ok()?;
                let url_to_test = url.join(&self.path_to_test).ok()?;
                Some(Mirror {
                    country: resolve_country(&m.country_code),
                    url,
                    url_to_test,
                })
            })
            .collect();

        Some(result)
    }

    /// Parse a plain pacman mirrorlist.
    fn parse_mirrorlist(&self, raw: &str) -> Vec<Mirror> {
        let mut current_country: Option<&'static Country> = None;
        let mut mirrors = Vec::new();

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('#') {
                if let Some(code) = country_code_from_line(trimmed) {
                    current_country = resolve_country(code);
                }
                continue;
            }

            let Some(rest) = trimmed.strip_prefix("Server = ") else {
                continue;
            };
            let cleaned = rest.replace("$arch/$repo", "");

            if let Ok(url) = Url::parse(&cleaned) {
                if let Ok(url_to_test) = url.join(&self.path_to_test) {
                    mirrors.push(Mirror {
                        country: current_country,
                        url,
                        url_to_test,
                    });
                }
            }
        }

        mirrors
    }
}

impl FetchMirrors for CachyOSTarget {
    fn fetch_mirrors(&self, tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let output = fetch_text_or_file(&self.mirror_list_file, self.fetch_mirrors_timeout)?;

        let result = match self.parse_api_json(&output) {
            Some(mirrors) => mirrors,
            None => self.parse_mirrorlist(&output),
        };

        tx_progress
            .send(format!("FETCHED MIRRORS: {}", result.len()))
            .unwrap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(max_delay: i64) -> CachyOSTarget {
        CachyOSTarget {
            fetch_mirrors_timeout: 15000,
            mirror_list_file: String::new(),
            path_to_test: "x86_64/cachyos/cachyos.files".to_string(),
            arch: "auto".to_string(),
            comment_prefix: "# ".to_string(),
            max_delay,
        }
    }

    #[test]
    fn country_code_from_line_extracts_token() {
        assert_eq!(country_code_from_line("## tier=1 code=US"), Some("US"));
        assert_eq!(country_code_from_line("## tier=2 code=RU"), Some("RU"));
        assert_eq!(country_code_from_line("## tier=1 code=AT"), Some("AT"));
        assert_eq!(
            country_code_from_line("## tier=1 code=GLOBAL"),
            Some("GLOBAL")
        );
    }

    #[test]
    fn country_code_from_line_returns_none_for_non_code_lines() {
        assert!(country_code_from_line("## USA Mirror much thanks to Soulharsh!").is_none());
        assert!(country_code_from_line("Server = https://example/").is_none());
    }

    #[test]
    fn resolve_country_handles_known_global_and_unknown() {
        assert_eq!(resolve_country("US").map(|c| c.code), Some("US"));
        assert_eq!(resolve_country("DE").map(|c| c.code), Some("DE"));
        assert!(resolve_country("GLOBAL").is_none());
        assert!(resolve_country("ABCD").is_none());
    }

    #[test]
    fn parses_api_json_with_country_and_delay_filter() {
        let target = make_target(86400);
        let raw = r#"{
            "mirrors": [
                {"country_code": "DE", "url": "https://de.example.org/repo/", "delay_seconds": 0},
                {"country_code": "GLOBAL", "url": "https://cdn.example.org/repo/", "delay_seconds": 10},
                {"country_code": "FR", "url": "https://fr.example.org/repo/", "delay_seconds": 999999},
                {"country_code": "US", "url": "https://us.example.org/repo/", "delay_seconds": null}
            ]
        }"#;

        let mirrors = target.parse_api_json(raw).expect("valid API json");

        assert_eq!(mirrors.len(), 3);
        assert!(
            mirrors
                .iter()
                .all(|m| m.url.as_str() != "https://fr.example.org/repo/")
        );

        let de = mirrors
            .iter()
            .find(|m| m.url.as_str() == "https://de.example.org/repo/")
            .expect("DE mirror kept");
        assert_eq!(de.country.map(|c| c.code), Some("DE"));
        assert_eq!(
            de.url_to_test.as_str(),
            "https://de.example.org/repo/x86_64/cachyos/cachyos.files"
        );

        // global unlabeled
        let global = mirrors
            .iter()
            .find(|m| m.url.as_str() == "https://cdn.example.org/repo/")
            .expect("GLOBAL mirror kept");
        assert!(global.country.is_none());
    }

    #[test]
    fn falls_back_to_plain_mirrorlist_with_sticky_country() {
        let target = make_target(86400);
        let raw = "\
## Some comment
## tier=1 code=DE
Server = https://de.example.org/repo/$arch/$repo
# Server = https://commented.example.org/repo/$arch/$repo
Server = https://fr.example.org/repo/$arch/$repo
";

        assert!(target.parse_api_json(raw).is_none());

        let mirrors = target.parse_mirrorlist(raw);
        assert_eq!(mirrors.len(), 2);
        assert_eq!(mirrors[0].url.as_str(), "https://de.example.org/repo/");
        assert_eq!(mirrors[0].country.map(|c| c.code), Some("DE"));
        assert_eq!(mirrors[1].url.as_str(), "https://fr.example.org/repo/");
        assert_eq!(mirrors[1].country.map(|c| c.code), Some("DE"));
    }

    #[test]
    fn mirrorlist_resets_country_on_global_code() {
        let target = make_target(86400);
        let raw = "\
## tier=1 code=DE
Server = https://de.example.org/repo/$arch/$repo
## tier=1 code=GLOBAL
Server = https://cdn.example.org/repo/$arch/$repo
";

        let mirrors = target.parse_mirrorlist(raw);
        assert_eq!(mirrors.len(), 2);
        assert_eq!(mirrors[0].country.map(|c| c.code), Some("DE"));
        assert!(mirrors[1].country.is_none());
    }
}
