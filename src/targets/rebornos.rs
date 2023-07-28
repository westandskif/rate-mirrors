use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::rebornos::RebornOSTarget;
use linkify::{LinkFinder, LinkKind};
use std::fmt::Display;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
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
        let url = "https://raw.githubusercontent.com/RebornOS-Developers/rebornos-mirrorlist/main/reborn-mirrorlist";

        let mirrorlist_file_text = Runtime::new().unwrap().block_on(async {
            Ok::<_, AppError>(
                reqwest::Client::new()
                    .get(url)
                    .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                    .send()
                    .await?
                    .text_with_charset("utf-16")
                    .await?,
            )
        })?;

        let mut link_finder = LinkFinder::new();
        link_finder.kinds(&[LinkKind::Url]);

        let mirrors: Vec<_> = link_finder
            .links(&mirrorlist_file_text)
            .filter_map(|url| Url::parse(url.as_str()).ok())
            .filter(|url| config.is_protocol_allowed_for_url(url))
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
