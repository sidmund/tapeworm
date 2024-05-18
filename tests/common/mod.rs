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

fn get_resource_path() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("resources")
        .join("test")
}

/// # Parameters
/// - `filename`: just the name of a file in the `resources/test` directory
/// - `to`: the directory to copy the file to
pub fn copy(filename: &str, to: &PathBuf) {
    fs::copy(get_resource_path().join(filename), to.join(filename)).unwrap();
}

pub fn create_lib(name: &str) -> PathBuf {
    let lib = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join(name);
    fs::create_dir_all(&lib).unwrap();
    lib
}

pub fn create_lib_with_folders(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let lib = create_lib(name);
    let lib_in = lib.join("in");
    let lib_out = lib.join("out");
    fs::create_dir_all(&lib_in).unwrap();
    fs::create_dir_all(&lib_out).unwrap();
    (lib, lib_in, lib_out)
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
