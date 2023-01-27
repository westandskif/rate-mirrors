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
use config::LogFormatter;
use itertools::Itertools;
use mirror::Mirror;
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

struct OutputSink<'a, T: LogFormatter> {
    file: Option<File>,
    formatter: &'a T,
    comments_enabled: bool,
}

impl<'a, T: LogFormatter> OutputSink<'a, T> {
    pub fn new(
        formatter: &'a T,
        filename: Option<&str>,
        comments_enabled: bool,
    ) -> Result<Self, io::Error> {
        let output = match filename {
            Some(filename) => {
                let file = File::create(String::from(filename))?;
                Self {
                    formatter,
                    file: Some(file),
                    comments_enabled,
                }
            }
            None => Self {
                formatter,
                file: None,
                comments_enabled,
            },
        };
        Ok(output)
    }

    pub fn display_comment(&mut self, line: impl Display) {
        if self.comments_enabled {
            self.write(self.formatter.format_comment(line))
        }
    }

    pub fn display_mirror(&mut self, mirror: &Mirror) {
        self.write(self.formatter.format_mirror(&mirror));
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

    let ref formatter = Arc::clone(&config).target;
    let mut output = OutputSink::new(
        formatter,
        config.save_to_file.as_deref(),
        !config.disable_comments,
    )?;

    output.display_comment(format!("STARTED AT: {}", Local::now()));
    output.display_comment(format!("ARGS: {}", env::args().join(" ")));

    let (tx_progress, rx_progress) = mpsc::channel::<String>();
    let (tx_results, rx_results) = mpsc::channel::<SpeedTestResults>();

    let thread_handle = thread::spawn(move || -> Result<(), AppError> {
        let mirrors = config
            .target
            .fetch_mirrors(Arc::clone(&config), tx_progress.clone())?;

        tx_progress
            .send(format!("MIRRORS LEFT AFTER FILTERING: {}", mirrors.len()))
            .unwrap();

        test_speed_by_countries(mirrors, config, tx_progress, tx_results);
        Ok(())
    });

    for progress in rx_progress.into_iter() {
        output.display_comment(progress);
    }

    thread_handle.join().unwrap()?;

    let results: Vec<_> = rx_results.iter().flatten().collect();

    output.display_comment("==== RESULTS (top re-tested) ====");

    for (index, result) in results.iter().enumerate() {
        output.display_comment(format!("{:>3}. {}", index + 1, result));
    }

    output.display_comment(format!("FINISHED AT: {}", Local::now()));

    let is_results_empty = results.is_empty();
    for result in results.into_iter() {
        output.display_mirror(&result.item);
    }

    // Fallback to unranked mirrors
    if is_results_empty {
        let fallback_config = Arc::new(Config::from_args());
        let (tx_fallback_progress, _) = mpsc::channel::<String>();
        let fallback_mirrors = fallback_config
            .target
            .fetch_mirrors(Arc::clone(&fallback_config), tx_fallback_progress.clone())?;
        for result in fallback_mirrors.into_iter() {
            output.display_mirror(&result);
        }
    }

    Ok(())
}
