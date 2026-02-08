use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::arch4edu::Arch4eduTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

const ARCH4EDU_MIRRORLIST_URL: &str =
    "https://raw.githubusercontent.com/arch4edu/mirrorlist/refs/heads/master/mirrorlist.arch4edu";

fn parse_mirror_url(line: &str) -> Option<Url> {
    let mut cleaned = line.trim_start();
    while cleaned.starts_with('#') {
        cleaned = cleaned.trim_start_matches('#').trim_start();
    }

    let raw_url = cleaned
        .strip_prefix("Global Server = ")
        .or_else(|| cleaned.strip_prefix("Server = "))?
        .trim()
        .replace("$arch", "");

    if raw_url.is_empty() {
        return None;
    }

    Url::parse(&raw_url).ok()
}

impl LogFormatter for Arch4eduTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}{}", mirror.url, arch)
    }
}

impl FetchMirrors for Arch4eduTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let output = fetch_text(ARCH4EDU_MIRRORLIST_URL, self.fetch_mirrors_timeout)?;

        let mirrors = output
            .lines()
            .filter_map(parse_mirror_url)
            .map(|url| {
                let url_to_test = url
                    .join(&self.path_to_test)
                    .expect("failed to join path_to_test");
                Mirror {
                    country: None,
                    url,
                    url_to_test,
                }
            })
            .collect();

        Ok(mirrors)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_mirror_url;
    use crate::config::LogFormatter;
    use crate::mirror::Mirror;
    use crate::target_configs::arch4edu::Arch4eduTarget;
    use url::Url;

    #[test]
    fn parse_server_line() {
        let url = parse_mirror_url("#Server = https://mirror.example/arch4edu/$arch").unwrap();
        assert_eq!(url.as_str(), "https://mirror.example/arch4edu/");
    }

    #[test]
    fn parse_global_server_line() {
        let url =
            parse_mirror_url("## Global Server = https://mirror.example/arch4edu/$arch").unwrap();
        assert_eq!(url.as_str(), "https://mirror.example/arch4edu/");
    }

    #[test]
    fn ignore_non_server_lines() {
        assert!(parse_mirror_url("# random comment").is_none());
        assert!(parse_mirror_url("Include = /etc/pacman.d/mirrorlist").is_none());
    }

    #[test]
    fn format_mirror_uses_arch_placeholder_for_auto() {
        let target = Arch4eduTarget {
            fetch_mirrors_timeout: 15_000,
            path_to_test: "arch4edu/x86_64/arch4edu.files".to_string(),
            arch: "auto".to_string(),
            comment_prefix: "# ".to_string(),
        };
        let mirror = Mirror {
            country: None,
            url: Url::parse("https://mirror.example/arch4edu/").unwrap(),
            url_to_test: Url::parse("https://mirror.example/arch4edu/x86_64/arch4edu.files")
                .unwrap(),
        };

        assert_eq!(
            target.format_mirror(&mirror),
            "Server = https://mirror.example/arch4edu/$arch"
        );
    }

    #[test]
    fn format_mirror_uses_custom_arch() {
        let target = Arch4eduTarget {
            fetch_mirrors_timeout: 15_000,
            path_to_test: "arch4edu/x86_64/arch4edu.files".to_string(),
            arch: "x86_64".to_string(),
            comment_prefix: "# ".to_string(),
        };
        let mirror = Mirror {
            country: None,
            url: Url::parse("https://mirror.example/arch4edu/").unwrap(),
            url_to_test: Url::parse("https://mirror.example/arch4edu/x86_64/arch4edu.files")
                .unwrap(),
        };

        assert_eq!(
            target.format_mirror(&mirror),
            "Server = https://mirror.example/arch4edu/x86_64"
        );
    }
}
