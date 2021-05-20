#[macro_use]
extern crate lazy_static;
extern crate reqwest;
mod config;
mod countries;
mod mirrors;
mod speed_test;
use crate::mirrors::MirrorData;
use crate::speed_test::{find_ones_with_top_speed, SpeedTestResult};
use chrono::prelude::*;
use config::Config;
use mirrors::fetch_mirrors;
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

struct Output {
    file: Option<File>,
}
impl Output {
    pub fn new(filename: Option<&str>) -> Result<Output, io::Error> {
        let output = match filename {
            Some(filename) => {
                let file = File::create(String::from(filename))?;
                Output { file: Some(file) }
            }
            None => Output { file: None },
        };
        Ok(output)
    }
    pub fn add_line<T>(&mut self, line: T)
    where
        T: AsRef<str> + Display,
    {
        let line = format!("{}\n", line);
        if let Some(file) = self.file.as_mut() {
            file.write_all(line.as_bytes()).unwrap();
        }
        print!("{}", &line);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if Uid::effective().is_root() {
        panic!("do not run rate-arch-mirrors with root permissions");
    }
    let config = Arc::new(Config::from_args());
    let mut output = Output::new(config.save_to_file.as_deref())?;
    output.add_line(format!("# STARTED AT: {}", Local::now()));
    output.add_line(format!(
        "# ARGS: {}",
        env::args().into_iter().collect::<Vec<String>>().join(" ")
    ));
    let (tx_progress, rx_progress) = mpsc::channel::<String>();
    let (tx_results, rx_results) = mpsc::channel::<Vec<SpeedTestResult<MirrorData>>>();

    let thread_handle = thread::spawn(move || {
        let main_tx_progress = mpsc::Sender::clone(&tx_progress);
        let map_result = fetch_mirrors(Arc::clone(&config), mpsc::Sender::clone(&tx_progress));
        match map_result {
            Ok(map) => {
                find_ones_with_top_speed(
                    Arc::clone(&config),
                    &map,
                    mpsc::Sender::clone(&tx_progress),
                    mpsc::Sender::clone(&tx_results),
                );
            }
            Err(error) => {
                main_tx_progress.send(format!("ERROR: {}", error)).unwrap();
            }
        }
    });

    for progress in rx_progress.into_iter() {
        output.add_line(format!("# {}", progress));
    }

    thread_handle.join().unwrap();
    let results = rx_results
        .iter()
        .flatten()
        .collect::<Vec<SpeedTestResult<MirrorData>>>();
    output.add_line(format!("# ==== RESULTS (top re-tested) ===="));

    for (index, result) in results.iter().enumerate() {
        output.add_line(format!(
            "# {:>3}. [{}] {} -> {}",
            index + 1,
            result.id.country_code,
            result,
            result.id.url
        ));
    }
    output.add_line(format!("# FINISHED AT: {}", Local::now()));

    for result in results.into_iter() {
        output.add_line(format!("Server = {}$repo/os/$arch", result.id.url));
    }
    Ok(())
}
