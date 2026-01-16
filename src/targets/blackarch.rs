use crate::config::{fetch_text, AppError, Config, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::blackarch::BlackArchTarget;
use std::fmt::Display;
use std::sync::{mpsc, Arc};
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

        let output = fetch_text(url, self.fetch_mirrors_timeout)?;

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
                    .join(&self.path_to_test)
                    .expect("failed to join path_to_test");
                Mirror {
                    country,
                    url,
                    url_to_test,
                }
            })
            .collect();

        Ok(mirrors)
    }
}
