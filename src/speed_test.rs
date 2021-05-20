extern crate byte_unit;
extern crate reqwest;
use crate::config::Config;
use crate::countries::{Country, LinkTo, LinkType};
use crate::mirrors::MirrorData;
use byte_unit::{Byte, ByteUnit};
use futures::future::join_all;
use itertools::Itertools;
use reqwest::Error as ReqwestError;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::convert::From;
use std::fmt;
use std::fmt::Debug;
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTimeError};
use tokio;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;

#[derive(Debug)]
pub struct SpeedTestResult<T> {
    pub bytes_downloaded: usize,
    pub elapsed: Duration,
    pub speed: f64,
    pub connection_time: Duration,
    pub id: T,
}
impl<T: Sized> SpeedTestResult<T> {
    pub fn new(
        id: T,
        bytes_downloaded: usize,
        elapsed: Duration,
        connection_time: Duration,
    ) -> SpeedTestResult<T> {
        SpeedTestResult {
            id: id,
            bytes_downloaded,
            elapsed,
            connection_time,
            speed: bytes_downloaded as f64 / elapsed.as_secs_f64(),
        }
    }

    #[inline]
    pub fn fmt_speed(&self) -> String {
        let speed = Byte::from_unit(self.speed, ByteUnit::B).unwrap();
        format!("{:.1}/s", speed.get_appropriate_unit(false))
    }
}
impl<T: Sized> fmt::Display for SpeedTestResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpeedTestResult {{ speed: {}; elapsed: {:?}; connection_time: {:?}}}",
            self.fmt_speed(),
            self.elapsed,
            self.connection_time,
        )
    }
}

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

fn rate_country_link(
    map: &HashMap<&Country, Vec<MirrorData>>,
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

fn test_mirrors_speed<'a, T>(
    mirrors: T,
    path_to_test: &'a str,
    per_mirror_timeout: Duration,
    min_per_mirror: Duration,
    min_bytes_per_mirror: usize,
    eps: f64,
    eps_checks: usize,
    runtime: &Runtime,
    semaphore: Arc<Semaphore>,
    tx_progress: mpsc::Sender<String>,
) -> Vec<SpeedTestResult<MirrorData>>
where
    T: IntoIterator<Item = MirrorData>,
{
    let mut handles = Vec::new();
    for mirror in mirrors.into_iter() {
        handles.push(runtime.spawn(mirror.test_speed(
            path_to_test.to_owned(),
            per_mirror_timeout,
            min_per_mirror,
            min_bytes_per_mirror,
            eps,
            eps_checks,
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        )));
    }
    runtime
        .block_on(join_all(handles))
        .into_iter()
        .filter_map(|r| r.ok())
        .filter_map(|r| r.ok())
        .collect()
}

pub fn find_ones_with_top_speed(
    config: Arc<Config>,
    map: &HashMap<&Country, Vec<MirrorData>>,
    tx_progress: mpsc::Sender<String>,
    tx_results: mpsc::Sender<Vec<SpeedTestResult<MirrorData>>>,
) {
    let entry = Country::from_str(config.entry_country.as_str()).unwrap();
    let max_jumps = config.max_jumps;
    let country_neighbors_per_country = config.country_neighbors_per_country;
    let country_test_mirrors_per_country = config.country_test_mirrors_per_country;
    let per_mirror_timeout = Duration::from_millis(config.per_mirror_timeout);
    let min_per_mirror = Duration::from_millis(config.min_per_mirror);
    let min_bytes_per_mirror = config.min_bytes_per_mirror;
    let eps = config.eps;
    let eps_checks = config.eps_checks;
    let concurrency = config.concurrency;
    let top_mirrors_number_to_retest = config.top_mirrors_number_to_retest;

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    let mut countries_to_check: Vec<&Country> = Vec::new();
    let mut speed_test_results: Vec<SpeedTestResult<MirrorData>> = Vec::new();
    let mut visited_countries: HashSet<&'static str> = HashSet::new();
    let mut explored_countries: HashSet<&'static str> = HashSet::new();
    let mut jumps_number: usize = 0;
    countries_to_check.push(entry);

    let mut latest_top_speeds: Vec<f64> = Vec::with_capacity(max_jumps);
    let mut latest_top_connection_times: Vec<Duration> = Vec::with_capacity(max_jumps);

    while countries_to_check.len() > 0 {
        tx_progress
            .send(format!("JUMP #{}", jumps_number + 1))
            .unwrap();
        let current_countries = countries_to_check;
        countries_to_check = Vec::new();

        let mirrors_to_check = current_countries
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
                let mirrors_of_country: Vec<MirrorData>;
                if !visited {
                    tx_progress
                        .send(format!("VISITED {}", country.code))
                        .unwrap();
                    visited_countries.insert(country.code);
                    mirrors_of_country = map
                        .get(country)
                        .map(|mirrors| {
                            mirrors
                                .iter()
                                .take(country_test_mirrors_per_country)
                                .cloned()
                                .collect::<Vec<MirrorData>>()
                        })
                        .into_iter()
                        .flatten()
                        .collect();
                } else {
                    mirrors_of_country = Vec::new();
                }

                let mut links: Vec<_> = if !explored {
                    country.links.iter().collect()
                } else {
                    Vec::new()
                };
                let mut mirrors_of_neighbors: Vec<_> = Vec::new();
                for strategy in [RateStrategy::DistanceFirst, RateStrategy::HubsFirst]
                    .iter()
                    .take(cmp::max(1, 3 - jumps_number as i8) as usize)
                    .rev()
                {
                    links.sort_unstable_by(|a, b| {
                        rate_country_link(map, &b, strategy)
                            .partial_cmp(&rate_country_link(map, &a, strategy))
                            .unwrap()
                    });
                    let mirrors = links
                        .iter()
                        .filter_map(|link| {
                            if !visited_countries.contains(link.code) {
                                let neighbor = Country::from_str(link.code);
                                if neighbor.is_none() {
                                    return None;
                                }
                                let neighbor = neighbor.unwrap();
                                visited_countries.insert(neighbor.code);
                                let mirrors = map
                                    .get(neighbor)
                                    .map(|mirrors| {
                                        mirrors
                                            .iter()
                                            .take(country_test_mirrors_per_country)
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
                        .take(country_neighbors_per_country)
                        .flatten();
                    for mirror in mirrors {
                        mirrors_of_neighbors.push(mirror);
                    }
                }
                mirrors_of_country
                    .into_iter()
                    .chain(mirrors_of_neighbors.into_iter())
                    .collect::<Vec<_>>()
            })
            .flatten();

        let mut results = test_mirrors_speed(
            mirrors_to_check,
            config.path_to_test.as_ref(),
            per_mirror_timeout,
            min_per_mirror,
            min_bytes_per_mirror,
            eps * 2.,
            eps_checks / 2,
            &runtime,
            Arc::clone(&semaphore),
            mpsc::Sender::clone(&tx_progress),
        );
        jumps_number += 1;

        if results.len() == 0 {
            tx_progress.send(format!("BLANK ITERATION")).unwrap();
            break;
        }

        results.sort_unstable_by(|a, b| a.connection_time.partial_cmp(&b.connection_time).unwrap());
        for (index, result) in results.iter().enumerate() {
            let top_country = Country::from_str(result.id.country_code.as_str()).unwrap();
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
            let top_country = Country::from_str(result.id.country_code.as_str()).unwrap();
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

        if jumps_number == max_jumps {
            break;
        }

        // === EARLY STOP CHECKS ===
        let connection_time_checks = 2;
        let speed_checks = 3;
        let speed_check_sensitivity = 1.2;
        let connection_time_check_sensitivity = 1.5;
        // BY CONNECTION TIME
        let connection_times_state = latest_top_connection_times
            .iter()
            .rev()
            .zip(latest_top_connection_times.iter().rev().skip(1))
            .map(|(next, prev)| {
                next.as_secs_f64() > prev.as_secs_f64() * connection_time_check_sensitivity
            })
            .take(connection_time_checks)
            .collect::<Vec<bool>>();
        if connection_times_state.len() == connection_time_checks
            && connection_times_state.iter().all(|b| *b)
        {
            tx_progress
                .send(format!("CONNECTION TIMES ARE GETTING WORSE, STOPPING"))
                .unwrap();
            break;
        }

        // BY SPEED
        let speeds_state = latest_top_speeds
            .iter()
            .rev()
            .zip(latest_top_speeds.iter().rev().skip(1))
            .map(|(next, prev)| *next as f64 * speed_check_sensitivity < *prev as f64)
            .take(speed_checks)
            .collect::<Vec<bool>>();
        if speeds_state.len() == speed_checks && speeds_state.iter().all(|b| *b) {
            tx_progress
                .send(format!("SPEEDS ARE GETTING WORSE, STOPPING"))
                .unwrap();
            break;
        }

        tx_progress.send(format!("")).unwrap();
    }

    tx_progress.send(format!("\n")).unwrap();
    if speed_test_results.len() == 0 {
        tx_progress.send(format!("NO RESULTS TO RE-TEST")).unwrap();
        return;
    } else {
        tx_progress.send(format!("RE-TESTING TOP MIRRORS")).unwrap();
    }

    let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    let mut other_results = speed_test_results.split_off(cmp::min(
        top_mirrors_number_to_retest,
        speed_test_results.len(),
    ));
    let top_mirrors = speed_test_results.into_iter().map(|result| result.id);

    let mut top_mirror_results = test_mirrors_speed(
        top_mirrors,
        config.path_to_test.as_ref(),
        per_mirror_timeout,
        min_per_mirror,
        min_bytes_per_mirror,
        eps,
        eps_checks,
        &runtime,
        Arc::clone(&semaphore),
        mpsc::Sender::clone(&tx_progress),
    );
    top_mirror_results.sort_by(|a, b| b.speed.partial_cmp(&a.speed).unwrap());
    top_mirror_results.append(&mut other_results);
    tx_results.send(top_mirror_results).unwrap();
    // TODO: test additional mirrors from top countries
}
