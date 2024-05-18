//! Integration testing helper functions.

use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use tapeworm::Config;

pub fn setup(mut args: Vec<&str>) -> Result<Config, Box<dyn std::error::Error>> {
    args.insert(0, "tapeworm");
    let args = args.into_iter().map(|s| String::from(s));
    Config::build(args)
}

/// Run the `config` and use `io::stdin` for reading any user input.
pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tapeworm::run(config, io::stdin().lock())
}

pub fn run_with<R: BufRead>(config: Config, reader: R) -> Result<(), Box<dyn std::error::Error>> {
    tapeworm::run(config, reader)
}

pub fn get_resources() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("resources")
        .join("test")
}

pub fn create_lib(name: &str) -> PathBuf {
    let lib = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join(name);
    fs::create_dir_all(&lib).unwrap();
    lib
}

/// Remove the library folder and all its contents.
pub fn destroy(lib: PathBuf) {
    fs::remove_dir_all(lib).unwrap();
}

pub fn read(path: PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

/// Write the `contents` to the file at `path`. If the file does not exist,
/// it is created; otherwise, it will be overwritten.
pub fn write(path: PathBuf, contents: String) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(contents.as_bytes()).unwrap();
}
