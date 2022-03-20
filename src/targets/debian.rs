use crate::config::{AppError, Config, FetchMirrors, LogFormatter};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::debian::{DebianTarget, SourceListEntriesOpts};
use itertools::Itertools;
use reqwest;
use select::document::Document;
use select::predicate::{Attr, Name};
use std::fmt::Display;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

pub fn format_debian_mirror(opts: &SourceListEntriesOpts, mirror: &Mirror) -> String {
    // The format for two one-line-style entries using the deb and deb-src types is:
    //   type [ option1=value1 option2=value2 ] uri suite [component1] [component2] [...]
    //
    //   deb [ arch=amd64,armel ] http://us.archive.ubuntu.com/ubuntu trusty main restricted

    let ref options = if opts.options.len() > 0 {
        format!(" [{}] ", opts.options.join(" "))
    } else {
        " ".to_string()
    };

    let ref components = opts.components.join(" ");

    opts.types
        .iter()
        .flat_map(|type_| {
            opts.suites.iter().map(move |suite| {
                format!(
                    "{}{}{} {} {}",
                    type_, options, mirror.url, suite, components
                )
            })
        })
        .join("\n")
}

impl LogFormatter for DebianTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format_debian_mirror(&self.source_list_opts, mirror)
    }
}

impl FetchMirrors for DebianTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        _tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://www.debian.org/mirror/mirrors_full";

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

        let document = Document::from(output.as_str());

        let content = document
            .find(Attr("id", "content"))
            .next()
            .ok_or(AppError::ParseError("mirror list".to_string()))?;

        let result: Vec<_> = content
            .find(Name("h3"))
            .flat_map(|head| -> Result<_, AppError> {
                let country = head.text();

                let mut mirrors = Vec::new();
                let mut current = head;

                loop {
                    current = match current.next() {
                        None => break,
                        Some(node) => node,
                    };

                    if current.is(Name("tt")) {
                        if let Some(link) = current.find(Name("a")).next() {
                            if link.text().contains("/debian/") {
                                mirrors.push(link.attr("href").unwrap());
                            }
                        }
                    }

                    if current.is(Name("h3")) {
                        break;
                    }
                }

                Ok(mirrors
                    .into_iter()
                    .filter_map(|url| Url::parse(url).ok())
                    .filter(|url| config.is_protocol_allowed_for_url(url))
                    .map(|url| Mirror {
                        country: Country::from_str(&country),
                        url_to_test: url.join(&self.path_to_test).unwrap(),
                        url,
                    })
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        Ok(result)
    }
}
