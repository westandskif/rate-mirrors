#[macro_use]
extern crate lazy_static;
extern crate reqwest;
mod config;
mod countries;
mod speed_test;
mod target_configs;
mod targets;
use crate::speed_test::{test_speed_by_countries, SpeedTestResult, SpeedTestResults};
use crate::targets::archlinux::fetch_arch_mirrors;
use crate::targets::manjaro::fetch_manjaro_mirrors;
use crate::targets::rebornos::fetch_rebornos_mirrors;
use crate::targets::stdin::read_mirrors;
use chrono::prelude::*;
use config::{Config, Target};
use nix::unistd::Uid;
use std::env;
use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use structopt::StructOpt;

struct OutputSink {
    file: Option<File>,
    comment_prefix: String,
}
impl OutputSink {
    fn new<T>(comment_prefix: T, filename: Option<&str>) -> Result<Self, io::Error>
    where
        T: AsRef<str>,
    {
        let comment_prefix = comment_prefix.as_ref().to_owned();
        let output = match filename {
            Some(filename) => {
                let file = File::create(String::from(filename))?;
                Self {
                    comment_prefix: comment_prefix,
                    file: Some(file),
                }
            }
            None => Self {
                comment_prefix: comment_prefix,
                file: None,
            },
        };
        Ok(output)
    }
    #[inline]
    fn _consume<T>(&mut self, line: T)
    where
        T: AsRef<str> + Display,
    {
        let line = line.as_ref();
        print!("{}", line);
        if let Some(f) = &mut self.file {
            f.write_all(line.as_bytes()).unwrap();
        }
    }
    fn consume<T>(&mut self, line: T)
    where
        T: AsRef<str> + Display,
    {
        self._consume(format!("{}\n", line));
    }
    fn consume_comment<T>(&mut self, line: T)
    where
        T: AsRef<str> + Display,
    {
        self._consume(format!("{}{}\n", &self.comment_prefix, line));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::from_args());
    if !config.allow_root && Uid::effective().is_root() {
        panic!("do not run rate-mirrors with root permissions");
    }
    let comment_prefix = match &config.target {
        Target::Arch(target) => &target.comment_prefix,
        Target::Stdin(target) => &target.comment_prefix,
        Target::Manjaro(target) => &target.comment_prefix,
        Target::RebornOS(target) => &target.comment_prefix,
    };
    let mut output = OutputSink::new(comment_prefix, config.save_to_file.as_deref())?;
    output.consume_comment(format!("STARTED AT: {}", Local::now()));
    output.consume_comment(format!(
        "ARGS: {}",
        env::args().into_iter().collect::<Vec<String>>().join(" ")
    ));
    let (tx_progress, rx_progress) = mpsc::channel::<String>();
    let (tx_results, rx_results) = mpsc::channel::<SpeedTestResults>();

    let thread_handle = thread::spawn(move || {
        let mirrors = match &config.target {
            Target::Arch(target) => fetch_arch_mirrors(
                Arc::clone(&config),
                target.clone(),
                mpsc::Sender::clone(&tx_progress),
            ),
            Target::Stdin(target) => read_mirrors(
                Arc::clone(&config),
                target.clone(),
                mpsc::Sender::clone(&tx_progress),
            ),
            Target::Manjaro(target) => fetch_manjaro_mirrors(
                Arc::clone(&config),
                target.clone(),
                mpsc::Sender::clone(&tx_progress),
            ),
            Target::RebornOS(target) => fetch_rebornos_mirrors(
                Arc::clone(&config),
                target.clone(),
                mpsc::Sender::clone(&tx_progress),
            ),
        };
        test_speed_by_countries(
            mirrors,
            Arc::clone(&config),
            mpsc::Sender::clone(&tx_progress),
            mpsc::Sender::clone(&tx_results),
        );
    });

    for progress in rx_progress.into_iter() {
        output.consume_comment(format!("{}", progress));
    }

    thread_handle.join().unwrap();
    let results = rx_results
        .iter()
        .map(|r| r.results)
        .flatten()
        .collect::<Vec<SpeedTestResult>>();
    output.consume_comment(format!("==== RESULTS (top re-tested) ===="));

    for (index, result) in results.iter().enumerate() {
        match result.item.country {
            Some(country) => {
                output.consume_comment(format!(
                    "{:>3}. [{}] {} -> {}",
                    index + 1,
                    country.code,
                    result,
                    &result.item.url
                ));
            }
            None => {
                output.consume_comment(format!(
                    "{:>3}. {} -> {}",
                    index + 1,
                    result,
                    &result.item.url
                ));
            }
        }
    }
    output.consume_comment(format!("FINISHED AT: {}", Local::now()));

    for result in results.into_iter() {
        output.consume(result.item.output);
    }
    Ok(())
}
