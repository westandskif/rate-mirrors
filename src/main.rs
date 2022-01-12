#[macro_use]
extern crate lazy_static;

mod config;
mod countries;
mod mirror;
mod speed_test;
mod target_configs;
mod targets;

use crate::config::{AppError, Config, FetchMirrors};
use crate::speed_test::{test_speed_by_countries, SpeedTestResults};
use chrono::prelude::*;
use config::Target;
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
    fn new(comment_prefix: &str, filename: Option<&str>) -> Result<Self, io::Error> {
        let comment_prefix = comment_prefix.to_string();
        let output = match filename {
            Some(filename) => {
                let file = File::create(String::from(filename))?;
                Self {
                    comment_prefix,
                    file: Some(file),
                }
            }
            None => Self {
                comment_prefix,
                file: None,
            },
        };
        Ok(output)
    }
    fn _consume(&mut self, line: impl Display) {
        print!("{}", line);
        if let Some(f) = &mut self.file {
            f.write_all(line.to_string().as_bytes()).unwrap();
        }
    }
    fn consume(&mut self, line: impl Display) {
        self._consume(format!("{}\n", line));
    }
    fn consume_comment(&mut self, line: impl Display) {
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
        Target::Artix(target) => &target.comment_prefix,
        Target::CachyOS(target) => &target.comment_prefix,
        Target::EndeavourOS(target) => &target.comment_prefix,
    };
    let mut output = OutputSink::new(comment_prefix, config.save_to_file.as_deref())?;
    output.consume_comment(format!("STARTED AT: {}", Local::now()));
    output.consume_comment(format!(
        "ARGS: {}",
        env::args().into_iter().collect::<Vec<String>>().join(" ")
    ));
    let (tx_progress, rx_progress) = mpsc::channel::<String>();
    let (tx_results, rx_results) = mpsc::channel::<SpeedTestResults>();

    let thread_handle = thread::spawn(move || -> Result<(), AppError> {
        let mirrors = config
            .target
            .fetch_mirrors(Arc::clone(&config), tx_progress.clone())?;
        test_speed_by_countries(mirrors, config, tx_progress, tx_results);
        Ok(())
    });

    for progress in rx_progress.into_iter() {
        output.consume_comment(progress.to_string());
    }

    thread_handle.join().unwrap()?;

    let results: Vec<_> = rx_results.iter().flatten().collect();

    output.consume_comment("==== RESULTS (top re-tested) ====".to_string());

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
        output.consume(result.item);
    }
    Ok(())
}
