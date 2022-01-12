use crate::config::{AppError, Config, FetchMirrors};
use crate::target_configs::stdin::StdinTarget;
use std::io::{self, BufRead};
use std::sync::{mpsc, Arc};

use crate::mirror::{Mirror, MirrorInfo};

impl FetchMirrors for StdinTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let mirrors: Vec<_> = io::stdin()
            .lock()
            .lines()
            .filter_map(
                |line| match MirrorInfo::parse(&line.unwrap(), &self.separator) {
                    Ok(info) => Some(Mirror {
                        country: info.country,
                        output: format!(
                            "{}{}",
                            self.output_prefix,
                            info.url
                                .join(&self.path_to_return)
                                .expect("failed to join path-to-return")
                        ),
                        url_to_test: info
                            .url
                            .join(&self.path_to_return)
                            .expect("failed to join path-to-return"),
                        url: info.url,
                    }),
                    Err(err) => {
                        eprintln!("{}", err);
                        None
                    }
                },
            )
            .filter(|mirror| config.is_protocol_allowed_for_url(&mirror.url))
            .collect();
        tx_progress
            .send(format!("READ {} MIRRORS FROM STDIN", mirrors.len()))
            .unwrap();
        Ok(mirrors)
    }
}
