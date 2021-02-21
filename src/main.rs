#[macro_use]
extern crate lazy_static;
extern crate reqwest;
mod config;
mod countries;
mod mirrors;
mod speed_test;
use config::Config;
use mirrors::fetch_mirrors;
use speed_test::find_ones_with_top_speed;
use std::sync::Arc;
use structopt::StructOpt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::from_args());

    let map = fetch_mirrors(Arc::clone(&config));
    let results = find_ones_with_top_speed(&map, Arc::clone(&config)).unwrap();
    println!("# ==== RESULTS (top re-tested) ====");
    for (index, result) in results.iter().enumerate() {
        println!(
            "# {:>3}. [{}] {} -> {}",
            index + 1,
            result.id.country_code,
            result,
            result.id.url
        );
    }
    for result in results.into_iter() {
        println!("Server = {}$repo/os/$arch", result.id.url);
    }
    Ok(())
}
