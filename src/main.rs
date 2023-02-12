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
    comments_in_file_enabled: bool,
}

impl<'a, T: LogFormatter> OutputSink<'a, T> {
    pub fn new(
        formatter: &'a T,
        filename: Option<&str>,
        comments_enabled: bool,
        comments_in_file_enabled: bool,
    ) -> Result<Self, io::Error> {
        let output = match filename {
            Some(filename) => {
                let file = File::create(String::from(filename))?;
                Self {
                    formatter,
                    file: Some(file),
                    comments_enabled,
                    comments_in_file_enabled,
                }
            }
            None => Self {
                formatter,
                file: None,
                comments_enabled,
                comments_in_file_enabled,
            },
        };
        Ok(output)
    }

    pub fn display_comment(&mut self, line: impl Display) {
        if self.comments_enabled {
            let s = self.formatter.format_comment(line);
            println!("{}", &s);
            if self.comments_in_file_enabled {
                if let Some(f) = &mut self.file {
                    writeln!(f, "{}", &s).unwrap();
                }
            }
        }
    }

    pub fn display_mirror(&mut self, mirror: &Mirror) {
        let s = self.formatter.format_mirror(&mirror);
        println!("{}", &s);
        if let Some(f) = &mut self.file {
            writeln!(f, "{}", &s).unwrap();
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
        !config.disable_comments_in_file,
    )?;

    output.display_comment(format!("STARTED AT: {}", Local::now()));
    output.display_comment(format!("ARGS: {}", env::args().join(" ")));

    let (tx_progress, rx_progress) = mpsc::channel::<String>();
    let (tx_results, rx_results) = mpsc::channel::<SpeedTestResults>();
    let (tx_mirrors, rx_mirrors) = mpsc::channel::<Mirror>();

    let thread_handle = thread::spawn(move || -> Result<(), AppError> {
        let mirrors = config
            .target
            .fetch_mirrors(Arc::clone(&config), tx_progress.clone())?;

        // sending untested mirrors back so we have a fallback in case if all tests fail
        for mirror in mirrors.iter().cloned() {
            tx_mirrors.send(mirror).unwrap();
        }

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

    if results.is_empty() {
        output.display_comment("==== FAILED TO TEST SPEEDS, RETURNING UNTESTED MIRRORS ====");
        for mirror in rx_mirrors.into_iter() {
            output.display_mirror(&mirror);
        }
    } else {
        output.display_comment("==== RESULTS (top re-tested) ====");

        for (index, result) in results.iter().enumerate() {
            output.display_comment(format!("{:>3}. {}", index + 1, result));
        }

        output.display_comment(format!("FINISHED AT: {}", Local::now()));

        for result in results.into_iter() {
            output.display_mirror(&result.item);
        }
    }

    Ok(())
}
