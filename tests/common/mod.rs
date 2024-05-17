//! Integration testing helper functions.

use std::env;
use std::path::PathBuf;
use tapeworm::Config;

pub fn setup(mut args: Vec<&str>) -> Result<Config, Box<dyn std::error::Error>> {
    args.insert(0, "tapeworm");
    let args = args.into_iter().map(|s| String::from(s));
    Config::build(args)
}

pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tapeworm::run(config)
}

pub fn get_resources() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("resources")
        .join("test")
}
