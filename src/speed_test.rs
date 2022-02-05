extern crate byte_unit;
extern crate reqwest;
use crate::config::Config;
use crate::countries::{Country, LinkTo, LinkType};
use crate::mirror::Mirror;
use byte_unit::{Byte, ByteUnit};
use futures::future::join_all;
use itertools::Itertools;
use reqwest::Error as ReqwestError;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::convert::From;
use std::fmt;
use std::fmt::Debug;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, SystemTimeError};

use tokio::runtime::Runtime;
use tokio::sync::Semaphore;

// #[derive(Clone)]
// pub struct SpeedTestConfig {
//     pub concurrency: usize,
//     pub path_to_test: String,
//     pub eps: f64,
//     pub eps_checks: usize,
//     pub max_jumps: usize,
//     pub min_bytes_per_mirror: usize,
//     pub min_per_mirror: Duration,
//     pub per_mirror_timeout: Duration,
//     pub top_mirrors_number_to_retest: usize,
// }
// #[derive(Clone)]
// pub struct SpeedTestByCountriesConfig {
//     pub base_config: Arc<SpeedTestConfig>,
//     pub country_neighbors_per_country: usize,
//     pub country_test_mirrors_per_country: usize,
//     pub entry_country: &'static Country,
// }

pub struct SpeedTestResult {
    pub bytes_downloaded: usize,
    pub elapsed: Duration,
    pub speed: f64,
    pub connection_time: Duration,
    pub item: Mirror,
}
impl SpeedTestResult {
    pub fn new(
        item: Mirror,
        bytes_downloaded: usize,
        elapsed: Duration,
        connection_time: Duration,
    ) -> SpeedTestResult {
        SpeedTestResult {
            item,
            bytes_downloaded,
            elapsed,
            connection_time,
            speed: bytes_downloaded as f64 / elapsed.as_secs_f64(),
        }
    }

    pub fn fmt_speed(&self) -> String {
        let speed = Byte::from_unit(self.speed, ByteUnit::B).unwrap();
        format!("{:.1}/s", speed.get_appropriate_unit(false))
    }
}

impl fmt::Debug for SpeedTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(country) = self.item.country {
            write!(f, "[{}] ", country.code)?;
        }
        write!(
            f,
            "SpeedTestResult {{ speed: {}; elapsed: {:?}; connection_time: {:?}}}",
            self.fmt_speed(),
            self.elapsed,
            self.connection_time,
        )
    }
}

impl fmt::Display for SpeedTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} -> {}", self, self.item.url)
    }
}

pub type SpeedTestResults = Vec<SpeedTestResult>;

#[derive(Debug)]
pub enum SpeedTestError {
    ReqwestError(String),
    SystemTimeError(String),
    TooFewBytesDownloadedError,
}
impl From<ReqwestError> for SpeedTestError {
    fn from(error: ReqwestError) -> Self {
        SpeedTestError::ReqwestError(format!("{:?}", error))
    }
}
impl From<SystemTimeError> for SpeedTestError {
    fn from(error: SystemTimeError) -> Self {
        SpeedTestError::SystemTimeError(format!("{:?}", error))
    }
}

#[derive(Debug)]
enum RateStrategy {
    HubsFirst,
    DistanceFirst,
}

async fn test_single_mirror(
    mirror: Mirror,
    config: Arc<Config>,
    semaphore: Arc<Semaphore>,
    tx_progress: Sender<String>,
) -> Result<SpeedTestResult, SpeedTestError> {
    let mut bytes_downloaded: usize = 0;

    let _permit = semaphore.acquire().await;

    let client = reqwest::Client::new();
    let started_connecting = SystemTime::now();
    let mut response = client
        .get(mirror.url_to_test.as_str())
        .timeout(Duration::from_millis(config.per_mirror_timeout))
        .send()
        .await?;
    let connection_time = started_connecting.elapsed().unwrap();
    let started_ts = SystemTime::now();
    let mut prev_ts = started_ts;
    let mut speeds: Vec<f64> = Vec::with_capacity(config.eps_checks);
    let mut index = 0;
    let eps_checks_f64 = config.eps_checks as f64;
    let mut filling_up = true;
    let min_per_mirror_duration = Duration::from_millis(config.min_per_mirror);
    while let Ok(Some(chunk)) = response.chunk().await {
        let chunk_size = chunk.len();
        bytes_downloaded += chunk_size;

        let now = SystemTime::now();
        let chunk_speed = chunk_size as f64 / now.duration_since(prev_ts).unwrap().as_secs_f64();
        prev_ts = now;

        if filling_up {
            speeds.push(chunk_speed);
            index = (index + 1) % config.eps_checks;
            if index == 0 {
                filling_up = false;
            }
        } else {
            speeds[index] = chunk_speed;
            index = (index + 1) % config.eps_checks;
        }
        if bytes_downloaded >= config.min_bytes_per_mirror
            && now.duration_since(started_ts).unwrap() > min_per_mirror_duration
            && speeds.len() == config.eps_checks
        {
            let mean = speeds.iter().sum::<f64>() / eps_checks_f64;
            let variance = speeds
                .iter()
                .map(|speed| {
                    let diff = mean - *speed;
                    diff * diff
                })
                .sum::<f64>()
                / eps_checks_f64;
            let std_deviation = variance.sqrt();

            if std_deviation / mean <= config.eps {
                break;
            }
        }
    }
    drop(_permit);

    if bytes_downloaded < config.min_bytes_per_mirror {
        return Err(SpeedTestError::TooFewBytesDownloadedError);
    }

    let speed_test_result = SpeedTestResult::new(
        mirror,
        bytes_downloaded,
        prev_ts
            .duration_since(started_ts)
            .unwrap_or_else(|_| Duration::from_millis(0)),
        connection_time,
    );

    tx_progress
        .send(format!("{:?}", speed_test_result))
        .unwrap();

    Ok(speed_test_result)
}

fn test_mirrors<T: IntoIterator<Item = Mirror>>(
    mirrors: T,
    config: Arc<Config>,
    runtime: &Runtime,
    semaphore: Arc<Semaphore>,
    tx_progress: mpsc::Sender<String>,
) -> SpeedTestResults {
    let mut handles = Vec::new();
    for mirror in mirrors.into_iter() {
        handles.push(runtime.spawn(test_single_mirror(
            mirror,
            Arc::clone(&config),
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        )));
    }

    runtime
        .block_on(join_all(handles))
        .into_iter()
        .filter_map(|r| r.ok())
        .filter_map(|r| {
            // // USEFUL FOR DEBUGGING
            // if let Err(e) = r.as_ref() {
            //     println!("DEBUG => {:#?}", e);
            // }
            r.ok()
        })
        .collect()
}

fn rate_country_link<T>(
    map: &HashMap<&Country, Vec<T>>,
    link: &LinkTo,
    strategy: &RateStrategy,
) -> f64 {
    let country = Country::from_str(link.code).unwrap();
    let mirrors_score = match map.get(country) {
        Some(mirrors) => mirrors.len(),
        None => 0,
    };
    let distance_score = match link.link_type {
        LinkType::Submarine => (1. / link.distance).powf(1.),
        LinkType::Terrestrial => (1. / link.distance).powf(0.9),
    } * 15000.;
    match strategy {
        RateStrategy::HubsFirst => {
            (country.cable_connections_number as f64 * 1000.
                + country.internet_exchanges_number as f64)
                * mirrors_score as f64
        }
        RateStrategy::DistanceFirst => distance_score * mirrors_score as f64,
    }
}

pub fn test_speed_by_countries(
    mirrors: Vec<Mirror>,
    config: Arc<Config>,
    tx_progress: mpsc::Sender<String>,
    tx_results: mpsc::Sender<SpeedTestResults>,
) {
    let mut map: HashMap<&'static Country, Vec<Mirror>> = HashMap::with_capacity(mirrors.len());
    let mut unlabeled_mirrors: Vec<Mirror> = Vec::new();
    for mirror in mirrors.into_iter() {
        match mirror.country {
            Some(country) => {
                map.entry(country).or_insert_with(Vec::new).push(mirror);
            }
            None => {
                unlabeled_mirrors.push(mirror);
            }
        }
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));

    let mut countries_to_check: Vec<&Country> = Vec::new();
    let mut speed_test_results: Vec<SpeedTestResult> = Vec::new();
    let mut tested_urls: HashSet<String> = HashSet::new();
    let mut visited_countries: HashSet<&'static str> = HashSet::new();
    let mut explored_countries: HashSet<&'static str> = HashSet::new();
    let mut jumps_number: usize = 0;

    let country = match Country::from_str(&config.entry_country) {
        Some(country) => country,
        None => {
            tx_progress
                .send("UNKNOWN entry_country, falling back to US".to_string())
                .unwrap();
            Country::from_str("US").unwrap()
        }
    };
    countries_to_check.push(country);

    let mut latest_top_speeds: Vec<f64> = Vec::with_capacity(config.max_jumps);
    let mut latest_top_connection_times: Vec<Duration> = Vec::with_capacity(config.max_jumps);

    while !countries_to_check.is_empty() {
        tx_progress
            .send(format!("JUMP #{}", jumps_number + 1))
            .unwrap();
        let current_countries = countries_to_check;
        countries_to_check = Vec::new();

        let mirrors_to_check: Vec<Mirror> = current_countries
            .into_iter()
            .map(|country| {
                let explored = explored_countries.contains(country.code);
                let visited = visited_countries.contains(country.code);
                if !explored {
                    tx_progress
                        .send(format!("EXPLORING {}", country.code))
                        .unwrap();
                    explored_countries.insert(country.code);
                }
                let mirrors_of_country = if visited {
                    Vec::new()
                } else {
                    tx_progress
                        .send(format!("VISITED {}", country.code))
                        .unwrap();
                    visited_countries.insert(country.code);
                    map.get(country)
                        .map(|mirrors| {
                            mirrors
                                .iter()
                                .take(config.country_test_mirrors_per_country)
                                .cloned()
                        })
                        .into_iter()
                        .flatten()
                        .collect()
                };

                let mut links: Vec<_> = if !explored {
                    country.links.iter().collect()
                } else {
                    Vec::new()
                };
                let mut mirrors_of_neighbors = Vec::new();
                for strategy in [RateStrategy::DistanceFirst, RateStrategy::HubsFirst]
                    .iter()
                    .take(cmp::max(1, 3 - jumps_number as i8) as usize)
                    .rev()
                {
                    links.sort_unstable_by(|a, b| {
                        rate_country_link(&map, b, strategy)
                            .partial_cmp(&rate_country_link(&map, a, strategy))
                            .unwrap()
                    });
                    let mirrors = links
                        .iter()
                        .filter_map(|link| {
                            if !visited_countries.contains(link.code) {
                                let neighbor = Country::from_str(link.code);
                                neighbor?;
                                let neighbor = neighbor.unwrap();
                                visited_countries.insert(neighbor.code);
                                let mirrors = map
                                    .get(neighbor)
                                    .map(|mirrors| {
                                        mirrors
                                            .iter()
                                            .take(config.country_test_mirrors_per_country)
                                            .cloned()
                                    })
                                    .filter(|mirrors| mirrors.len() > 0);
                                if mirrors.is_some() {
                                    tx_progress
                                        .send(format!(
                                            "    + NEIGHBOR {} (by {:?})",
                                            link.code, strategy
                                        ))
                                        .unwrap();
                                    return mirrors;
                                }
                            }
                            None
                        })
                        .take(config.country_neighbors_per_country)
                        .flatten();
                    for mirror in mirrors {
                        mirrors_of_neighbors.push(mirror);
                    }
                }
                mirrors_of_country
                    .into_iter()
                    .chain(mirrors_of_neighbors.into_iter())
            })
            .flatten()
            .collect();

        tested_urls.extend(mirrors_to_check.iter().map(|m| m.url_to_test.to_string()));

        let mut results = test_mirrors(
            mirrors_to_check,
            Arc::clone(&config),
            &runtime,
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        );
        jumps_number += 1;

        if results.is_empty() {
            tx_progress.send("BLANK ITERATION".to_string()).unwrap();
            break;
        }

        results.sort_unstable_by(|a, b| a.connection_time.partial_cmp(&b.connection_time).unwrap());
        for (index, result) in results.iter().enumerate() {
            let top_country = result.item.country.unwrap();
            let is_neighbor = !explored_countries.contains(top_country.code);
            if is_neighbor {
                tx_progress
                    .send(format!(
                        "    TOP NEIGHBOR - CONNECTION TIME: {} - {:?}",
                        top_country.code, result.connection_time,
                    ))
                    .unwrap();
                countries_to_check.push(top_country);
                latest_top_connection_times.push(result.connection_time);
                break;
            } else if index == 0 {
                tx_progress
                    .send(format!(
                        "    TOP CONNECTION TIME: {} - {:?}",
                        top_country.code, result.connection_time,
                    ))
                    .unwrap();
                latest_top_connection_times.push(result.connection_time);
            }
        }

        results.sort_unstable_by(|a, b| b.speed.partial_cmp(&a.speed).unwrap());
        for (index, result) in results.iter().enumerate() {
            let top_country = result.item.country.unwrap();
            let is_neighbor = !explored_countries.contains(top_country.code);
            if is_neighbor {
                tx_progress
                    .send(format!(
                        "    TOP NEIGHBOR - SPEED: {} - {}",
                        top_country.code,
                        result.fmt_speed(),
                    ))
                    .unwrap();
                countries_to_check.push(top_country);
                latest_top_speeds.push(result.speed);
                break;
            } else if index == 0 {
                tx_progress
                    .send(format!(
                        "    TOP SPEED: {} - {}",
                        top_country.code,
                        result.fmt_speed(),
                    ))
                    .unwrap();
                latest_top_speeds.push(result.speed);
            }
        }

        speed_test_results = speed_test_results
            .into_iter()
            .merge_by(results.into_iter(), |a, b| a.speed > b.speed)
            .collect();

        if jumps_number == config.max_jumps {
            break;
        }

        // === EARLY STOP CHECKS ===
        let connection_time_checks = 2;
        let speed_checks = 3;
        let speed_check_sensitivity = 1.2;
        let connection_time_check_sensitivity = 1.5;
        // BY CONNECTION TIME
        let connection_times_state: Vec<bool> = latest_top_connection_times
            .iter()
            .rev()
            .zip(latest_top_connection_times.iter().rev().skip(1))
            .map(|(next, prev)| {
                next.as_secs_f64() > prev.as_secs_f64() * connection_time_check_sensitivity
            })
            .take(connection_time_checks)
            .collect();
        if connection_times_state.len() == connection_time_checks
            && connection_times_state.iter().all(|b| *b)
        {
            tx_progress
                .send("CONNECTION TIMES ARE GETTING WORSE, STOPPING".to_string())
                .unwrap();
            break;
        }

        // BY SPEED
        let speeds_state: Vec<bool> = latest_top_speeds
            .iter()
            .rev()
            .zip(latest_top_speeds.iter().rev().skip(1))
            .map(|(next, prev)| *next as f64 * speed_check_sensitivity < *prev as f64)
            .take(speed_checks)
            .collect();
        if speeds_state.len() == speed_checks && speeds_state.iter().all(|b| *b) {
            tx_progress
                .send("SPEEDS ARE GETTING WORSE, STOPPING".to_string())
                .unwrap();
            break;
        }

        tx_progress.send(format!("")).unwrap();
    }

    if speed_test_results.len()
        < config.max_jumps
            * config.country_test_mirrors_per_country
            * config.country_neighbors_per_country
    {
        tx_progress
            .send(format!(
                "COUNTRY JUMPING YIELDED TOO FEW MIRRORS ({}), ADDING OTHERS TO UNLABELED",
                speed_test_results.len()
            ))
            .unwrap();
        for mirrors in map.into_values() {
            let mut untested_mirrors: Vec<Mirror> = mirrors
                .into_iter()
                .filter(|m| !tested_urls.contains(m.url_to_test.as_str()))
                .collect();
            unlabeled_mirrors.append(&mut untested_mirrors);
        }
    }

    if !unlabeled_mirrors.is_empty() {
        tx_progress.send("\n".to_string()).unwrap();
        tx_progress
            .send("TESTING UNLABELED MIRRORS".to_string())
            .unwrap();

        let mut results = test_mirrors(
            unlabeled_mirrors,
            Arc::clone(&config),
            &runtime,
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        );

        results.sort_unstable_by(|a, b| b.speed.partial_cmp(&a.speed).unwrap());
        speed_test_results = speed_test_results
            .into_iter()
            .merge_by(results.into_iter(), |a, b| a.speed > b.speed)
            .collect();
    }

    tx_progress.send("\n".to_string()).unwrap();
    if speed_test_results.is_empty() {
        tx_progress
            .send("NO RESULTS TO RE-TEST".to_string())
            .unwrap();
        return;
    } else {
        tx_progress
            .send("RE-TESTING TOP MIRRORS".to_string())
            .unwrap();
    }

    let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    let mut other_results = speed_test_results.split_off(cmp::min(
        config.top_mirrors_number_to_retest,
        speed_test_results.len(),
    ));
    let top_mirrors = speed_test_results.into_iter().map(|result| result.item);

    let mut top_mirror_results = test_mirrors(
        top_mirrors,
        config,
        &runtime,
        Arc::clone(&semaphore),
        mpsc::Sender::clone(&tx_progress),
    );
    top_mirror_results.sort_by(|a, b| b.speed.partial_cmp(&a.speed).unwrap());
    top_mirror_results.append(&mut other_results);
    tx_results.send(top_mirror_results).unwrap();
}
