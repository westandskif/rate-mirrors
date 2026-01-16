use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::target_configs::stdin::StdinTarget;
use std::fmt::Display;
use std::io::{self, BufRead};
use std::sync::{mpsc, Arc};

use crate::mirror::{Mirror, MirrorInfo};

impl LogFormatter for StdinTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!(
            "{}{}",
            self.output_prefix,
            mirror
                .url
                .join(&self.path_to_return)
                .expect("failed to join path-to-return")
        )
    }
}

impl FetchMirrors for StdinTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        _tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let mirrors: Vec<_> = io::stdin()
            .lock()
            .lines()
            .filter_map(
                |line| match line {
                    Ok(line) => match MirrorInfo::parse(&line, &self.separator) {
                        Ok(info) => Some(Mirror {
                            country: info.country,
                            url_to_test: info
                                .url
                                .join(&self.path_to_test)
                                .expect("failed to join path-to-test"),
                            url: info.url,
                        }),
                        Err(err) => {
                            eprintln!("{}", err);
                            None
                        }
                    },
                    Err(err) => {
                        eprintln!("failed to read line: {}", err);
                        None
                    }
                },
            )
            .filter(|mirror| config.is_protocol_allowed_for_url(&mirror.url))
            .collect();

        Ok(mirrors)
    }
}
