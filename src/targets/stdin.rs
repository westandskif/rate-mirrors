use crate::config::{Config, Protocol};
use crate::countries::Country;
use crate::target_configs::stdin::StdinTarget;
use std::fmt;
use std::io::{self, BufRead};
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use url::Url;

#[derive(Clone, Debug)]
pub struct Mirror {
    pub country: Option<&'static Country>,
    pub output: String,
    pub url: Url,
    pub url_to_test: Url,
}

impl fmt::Display for Mirror {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.country {
            Some(country) => {
                write!(f, "[{}] {}", country.code, &self.url)
            }
            None => {
                write!(f, "{}", &self.url)
            }
        }
    }
}

impl Mirror {
    pub fn line_to_mirror_info<T>(line: T) -> Result<(Url, Option<&'static Country>), &'static str>
    where
        T: AsRef<str>,
    {
        let args: Vec<&str> = line.as_ref().trim().split("\t").collect();
        match args.len() {
            1 => match Url::parse(args[0]) {
                Ok(url) => Ok((url, None)),
                Err(_) => {
                    eprintln!("skipping input line: bad url {}", args.join("\t"));
                    return Err("bad url");
                }
            },
            2 => {
                let country_index;
                let url = match Url::parse(args[0]) {
                    Ok(url) => {
                        country_index = 1;
                        url
                    }
                    Err(_) => match Url::parse(args[1]) {
                        Ok(url) => {
                            country_index = 0;
                            url
                        }
                        Err(_) => {
                            eprintln!("skipping input line: bad url {}", args.join("\t"));
                            return Err("bad url");
                        }
                    },
                };
                let country = Country::from_str(args[country_index]);
                if let None = country {
                    eprintln!(
                        "unknown country -> {}; considering as None",
                        args[country_index]
                    );
                }
                Ok((url, country))
            }
            _ => {
                eprintln!("skipping bad input line: {}", args.join("\t"));
                return Err("bad input line");
            }
        }
    }
}

pub fn read_mirrors(
    config: Arc<Config>,
    target: StdinTarget,
    tx_progress: mpsc::Sender<String>,
) -> Vec<Mirror> {
    let fallback_protocols;
    let allowed_protocols: &[Protocol] = match config.protocols.len() {
        0 => {
            fallback_protocols = vec![Protocol::Http, Protocol::Https];
            &fallback_protocols
        }
        _ => &config.protocols,
    };
    let mirrors: Vec<Mirror> = io::stdin()
        .lock()
        .lines()
        .filter_map(|line| Mirror::line_to_mirror_info(line.unwrap()).ok())
        .filter_map(|(url, country)| {
            Some(Mirror {
                country,
                output: url
                    .join(&target.path_to_return)
                    .expect("failed to join path-to-return")
                    .as_str()
                    .to_owned(),
                url_to_test: url
                    .join(&target.path_to_test)
                    .expect("failed to join path-to-test"),
                url,
            })
        })
        .filter(|mirror| {
            allowed_protocols.contains(&Protocol::from_str(mirror.url.scheme()).unwrap())
        })
        .collect();
    tx_progress
        .send(format!("READ {} MIRRORS FROM STDIN", mirrors.len()))
        .unwrap();
    mirrors
}
