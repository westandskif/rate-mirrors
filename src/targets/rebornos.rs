use crate::config::{fetch_text, AppError, Config, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::rebornos::RebornOSTarget;
use std::fmt::Display;
use std::sync::{mpsc, Arc};
use url::Url;

impl LogFormatter for RebornOSTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}", mirror.url)
    }
}

impl FetchMirrors for RebornOSTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        _tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://raw.githubusercontent.com/RebornOS-Team/rebornos-mirrorlist/main/reborn-mirrorlist";

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

        let urls: Vec<Url> = output
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.replace("Server = ", ""))
            .filter(|line| !line.is_empty())
            .filter_map(|line| Url::parse(&line).ok())
            .filter(|url| {
                url.scheme()
                    .parse()
                    .map(|p| config.is_protocol_allowed(&p))
                    .unwrap_or(false)
            })
            .collect();

        let mirrors: Vec<_> = urls
            .into_iter()
            .map(|url| Mirror {
                country: None,
                url_to_test: url
                    .join(&self.path_to_test)
                    .expect("failed to join path-to-test"),
                url,
            })
            .collect();

        Ok(mirrors)
    }
}
