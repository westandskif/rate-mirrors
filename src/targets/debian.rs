use crate::config::{AppError, Config, FetchMirrors};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::debian::{DebianTarget, SourceListEntriesOpts};
use itertools::Itertools;
use reqwest;
use select::document::Document;
use select::predicate::{Attr, Name};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

pub fn display_mirror(target: &SourceListEntriesOpts, url: &Url) -> String {
    // The format for two one-line-style entries using the deb and deb-src types is:
    //   type [ option1=value1 option2=value2 ] uri suite [component1] [component2] [...]
    //
    //   deb [ arch=amd64,armel ] http://us.archive.ubuntu.com/ubuntu trusty main restricted

    let ref options = if target.options.len() > 0 {
        format!(" [{}] ", target.options.join(" "))
    } else {
        " ".to_string()
    };

    let ref components = target.components.join(" ");

    target
        .types
        .iter()
        .flat_map(|type_| {
            target
                .suites
                .iter()
                .map(move |suite| format!("{}{}{} {} {}", type_, options, url, suite, components))
        })
        .join("\n")
}

impl FetchMirrors for DebianTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
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

                // println!("[{}] {:?}", country, mirrors);

                Ok(mirrors
                    .into_iter()
                    .filter_map(|url| Url::parse(url).ok())
                    .filter(|url| config.is_protocol_allowed_for_url(url))
                    .map(|url| Mirror {
                        country: Country::from_str(&country),
                        output: display_mirror(&self.source_list_opts, &url),
                        url_to_test: url.join(&self.path_to_test).unwrap(),
                        url,
                    })
                    .collect::<Vec<_>>())
            })
            .flatten()
            .collect();

        tx_progress
            .send(format!("MIRRORS LEFT AFTER FILTERING: {}", result.len()))
            .unwrap();

        Ok(result)
    }
}
