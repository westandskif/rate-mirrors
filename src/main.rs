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
use itertools::Itertools;
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

pub struct LogFormatter {
    comment_prefix: String,
}

impl LogFormatter {
    pub fn new(comment_prefix: &str) -> Self {
        Self {
            comment_prefix: comment_prefix.to_string(),
        }
    }
    pub fn debug(&self, s: impl Display) -> impl Display {
        format!("{}{}", self.comment_prefix, s)
    }

    pub fn info(&self, s: impl Display) -> impl Display {
        s
    }
}

struct OutputSink {
    file: Option<File>,
    formatter: LogFormatter,
}

impl OutputSink {
    pub fn new(formatter: LogFormatter, filename: Option<&str>) -> Result<Self, io::Error> {
        let output = match filename {
            Some(filename) => {
                let file = File::create(String::from(filename))?;
                Self {
                    formatter,
                    file: Some(file),
                }
            }
            None => Self {
                formatter,
                file: None,
            },
        };
        Ok(output)
    }
    pub fn debug(&mut self, line: impl Display) {
        self.write(self.formatter.debug(line))
    }
    pub fn info(&mut self, line: impl Display) {
        self.write(self.formatter.info(line))
    }
    fn write(&mut self, line: impl Display) {
        println!("{}", line);
        if let Some(f) = &mut self.file {
            writeln!(f, "{}", line).unwrap();
        }
    }
}

fn main() -> Result<(), AppError> {
    let config = Arc::new(Config::from_args());
    if !config.allow_root && Uid::effective().is_root() {
        return Err(AppError::Root);
    }

    let mut output = OutputSink::new(
        config.target.get_formatter(),
        config.save_to_file.as_deref(),
    )?;

    output.debug(format!("STARTED AT: {}", Local::now()));
    output.debug(format!("ARGS: {}", env::args().join(" ")));

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
        output.debug(progress);
    }

    thread_handle.join().unwrap()?;

    let results: Vec<_> = rx_results.iter().flatten().collect();

    output.debug("==== RESULTS (top re-tested) ====");

    for (index, result) in results.iter().enumerate() {
        output.debug(format!("{:>3}. {}", index + 1, result));
    }

    output.debug(format!("FINISHED AT: {}", Local::now()));

    for result in results.into_iter() {
        output.info(result.item);
    }

    Ok(())
}
