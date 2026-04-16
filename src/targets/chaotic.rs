use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::chaotic::ChaoticTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

fn parse_country_code(line: &str) -> Option<&str> {
    let inside = line.trim_end().strip_suffix(')')?;
    let (_, code) = inside.rsplit_once('(')?;
    if code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase()) {
        Some(code)
    } else {
        None
    }
}

impl LogFormatter for ChaoticTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}$repo/{}", mirror.url, arch)
    }
}

impl FetchMirrors for ChaoticTarget {
    fn fetch_mirrors(&self, tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let url = "https://gitlab.com/chaotic-aur/pkgbuilds/-/raw/main/chaotic-mirrorlist/mirrorlist";

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

        let mut current_country = None;
        let mut mirrors = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim_start();
            let stripped = trimmed.strip_prefix('#').map(str::trim_start);

            // Parse country from lines like "# Australia (AU)" — requires a trailing
            // 2-letter uppercase code so attribution/explanatory comments don't match.
            if let Some(code) = stripped.and_then(parse_country_code) {
                current_country = Country::from_str(code);
                continue;
            }

            let uncommented = stripped.unwrap_or(trimmed);
            let Some(rest) = uncommented.strip_prefix("Server = ") else {
                continue;
            };
            let cleaned = rest.replace("$repo/$arch", "");

            match Url::parse(&cleaned) {
                Ok(url) => mirrors.push(Mirror {
                    country: current_country,
                    url_to_test: url
                        .join(&self.path_to_test)
                        .expect("failed to join path_to_test"),
                    url,
                }),
                Err(e) => {
                    tx_progress
                        .send(format!("chaotic: skipping unparseable URL {}: {}", cleaned, e))
                        .ok();
                }
            }
        }

        Ok(mirrors)
    }
}
