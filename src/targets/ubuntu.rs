use crate::config::{AppError, Config, FetchMirrors};
use crate::countries::Country;
use crate::mirror::Mirror;
use crate::target_configs::ubuntu::UbuntuTarget;
use crate::targets::debian::display_mirror;
use reqwest;
use select::document::Document;
use select::node::{Data, Node};
use select::predicate::{Attr, Class, Name, Predicate};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

#[derive(Debug)]
struct UbuntuMirrorInfo {
    urls: Vec<Url>,
    is_up_to_date: bool,
}

fn parse_header(node: &Node) -> Result<(String, usize), AppError> {
    let columns: Vec<_> = node.find(Name("th")).collect();
    let country = columns
        .first()
        .ok_or(AppError::ParseError("country".to_string()))?
        .text();

    let n = columns
        .last()
        .ok_or(AppError::ParseError("number of mirrors".to_string()))?
        .first_child()
        .ok_or(AppError::ParseError("number of mirrors".to_string()))?
        .text()
        .trim()
        .parse()
        .map_err(|_e| AppError::ParseError("number of mirrors".to_string()))?;

    Ok((country, n))
}

fn parse_mirror_info(node: &Node) -> Result<UbuntuMirrorInfo, AppError> {
    let columns: Vec<_> = node.find(Name("td")).collect();

    let urls = columns
        .get(1)
        .ok_or(AppError::ParseError("mirror row".to_string()))?
        .find(Name("a"))
        .filter_map(|x| x.attr("href"))
        .filter_map(|x| x.parse().ok())
        .collect();

    let state = columns
        .last()
        .ok_or(AppError::ParseError("mirror state column".to_string()))?
        .text()
        .trim()
        .to_lowercase();

    Ok(UbuntuMirrorInfo {
        urls,
        is_up_to_date: state == "up to date",
    })
}

impl FetchMirrors for UbuntuTarget {
    fn fetch_mirrors(
        &self,
        config: Arc<Config>,
        tx_progress: mpsc::Sender<String>,
    ) -> Result<Vec<Mirror>, AppError> {
        let url = "https://launchpad.net/ubuntu/+archivemirrors";

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

        let table = document
            .find(Attr("id", "mirrors_list"))
            .next()
            .ok_or(AppError::ParseError("mirror list table".to_string()))?;

        let result: Vec<_> = table
            .find(Name("tr").and(Class("head")))
            .flat_map(|head| -> Result<_, AppError> {
                // the next <n> rows belongs to <country>
                let (country, n) = parse_header(&head)?;

                let mut mirrors = Vec::with_capacity(n);
                let mut current = head;

                loop {
                    current = match current.next() {
                        None => break,
                        Some(node) => node,
                    };

                    if matches!(current.data(), Data::Element(_, _)) {
                        mirrors.push(parse_mirror_info(&current)?);
                    }
                    if mirrors.len() == n || current.is(Class("section-break")) {
                        break;
                    }
                }

                Ok(mirrors
                    .into_iter()
                    .filter(|m| m.is_up_to_date)
                    .filter_map(|m| {
                        let url = config.get_preferred_url(&m.urls)?;

                        Some(Mirror {
                            country: Country::from_str(&country),
                            output: display_mirror(&self.source_list_opts, url),
                            url: url.clone(),
                            url_to_test: url.join(&self.path_to_test).unwrap(),
                        })
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
