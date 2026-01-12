use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::blackarch::BlackArchTarget;
use reqwest;
use std::fmt::Display;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

impl LogFormatter for BlackArchTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let arch = if self.arch == "auto" {
            "$arch"
        } else {
            &self.arch
        };

        format!("Server = {}$repo/os/{}", mirror.url, arch)
    }
}

impl FetchMirrors for BlackArchTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        _tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://raw.githubusercontent.com/BlackArch/blackarch/master/mirror/mirror.lst";

        // RU|http://mirror.surf/blackarch/$repo/os/$arch|mirror.surf
        //
        // http://mirror.surf/blackarch/blackarch/os/x86_64/blackarch.files

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

        let mirrors: Vec<Mirror> = output
            .lines()
            .filter_map(|line| {
                if line.starts_with("#") {
                    return None;
                }
                let pieces: Vec<&str> = line.split("|").collect();
                if pieces.len() < 2 {
                    return None;
                }
                let country = Country::from_str(pieces[0]);
                match Url::parse(&pieces[1].replace("$repo/os/$arch", "")).ok() {
                    Some(url) => Some((url, country)),
                    None => None,
                }
            })
            .filter(|(url, _)| config.is_protocol_allowed_for_url(url))
            .map(|(url, country)| {
                let url_to_test = url
                    .join(&format!("{}.files", self.base_path))
                    .expect("failed to join path_to_test");
                Mirror {
                    country,
                    url,
                    url_to_test,
                    base_path: Some(self.base_path.clone()),
                }
            })
            .collect();

        Ok(mirrors)
    }
}
