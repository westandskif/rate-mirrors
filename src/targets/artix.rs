use crate::config::{AppError, Config, FetchMirrors};
use crate::mirror::Mirror;
use crate::target_configs::artix::ArtixTarget;
use reqwest;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

impl FetchMirrors for ArtixTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://gitea.artixlinux.org/packagesA/artix-mirrorlist/raw/branch/master/trunk/mirrorlist";

        let output = Runtime::new().unwrap().block_on(async {
            Ok::<_, AppError>(
                reqwest::Client::new()
                    .get(url)
                    .timeout(Duration::from_millis(self.fetch_mirrors_timeout))
                    .send()
                    .await?
                    .text_with_charset("utf-8")
                    .await?,
            )
        })?;

        let urls = output
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.replace("Server = ", "").replace("$repo/os/$arch", ""))
            .filter(|line| !line.is_empty())
            .filter_map(|line| Url::parse(&line).ok())
            .filter(|url| {
                url.scheme()
                    .parse()
                    .map(|p| config.is_protocol_allowed(&p))
                    .unwrap_or(false)
            });

        let result: Vec<_> = urls
            .map(|url| {
                let url_to_test = url
                    .join(&self.path_to_test)
                    .expect("failed to join path_to_test");

                Mirror {
                    country: None,
                    output: format!("Server = {}$repo/os/$arch", url),
                    url_to_test,
                    url,
                }
            })
            .collect();

        tx_progress
            .send(format!("MIRRORS LEFT AFTER FILTERING: {}", result.len()))
            .unwrap();

        Ok(result)
    }
}
