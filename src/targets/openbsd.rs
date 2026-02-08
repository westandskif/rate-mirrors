use crate::config::{fetch_text, AppError, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::openbsd::OpenBSDTarget;
use std::fmt::Display;
use std::sync::mpsc;
use url::Url;

impl LogFormatter for OpenBSDTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("{}", mirror.url)
    }
}

impl FetchMirrors for OpenBSDTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        let url = "https://ftp.openbsd.org/pub/OpenBSD/ftplist";

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

        let urls = output
            .lines()
            .map(|line| {
                let url_part = line.split_whitespace().next().unwrap_or("");
                let description_part = line.get(url_part.len()..).map_or("", |s| s.trim_start());
                (url_part, description_part)
            })
            .filter(|(url_part, description_part)| {
                !url_part.is_empty() && !description_part.is_empty()
            })
            .filter_map(|(url_part, description_part)| {
                Url::parse(&url_part)
                    .ok()
                    .map(|url| (url, description_part))
            });

        let result: Vec<_> = urls
            .filter_map(|(url, description_part)| {
                let country = {
                    if description_part.ends_with(" (CDN)") {
                        "CDN"
                    } else if description_part.ends_with(", The Netherlands") {
                        "NL"
                    } else if description_part.ends_with(", USA") {
                        "US"
                    } else if let Some(comma_pos) = description_part.rfind(',') {
                        let potential_country = description_part[comma_pos + 1..].trim();
                        if !potential_country.is_empty() {
                            potential_country
                        } else {
                            description_part
                        }
                    } else {
                        description_part
                    }
                };
                Url::parse((url.to_string() + self.path_to_test.as_str()).as_ref())
                    .ok()
                    .map(|url_to_test| Mirror {
                        country: Country::from_str(country),
                        url,
                        url_to_test,
                    })
            })
            .collect();

        Ok(result)
    }
}
