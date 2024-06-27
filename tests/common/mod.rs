//! Integration testing helper functions.

use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::{env, fs};
use tapeworm::{Config, Downloader};

/// Mocks yt-dlp by simply creating a file for each input.
pub struct MockYtDlp;
impl Downloader for MockYtDlp {
    fn download<R: BufRead>(
        &self,
        config: &Config,
        inputs: HashSet<String>,
        _reader: R,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dest = config.lib_path.as_ref().unwrap().join(".tapeworm").join("in");
        for (i, input) in inputs.iter().map(|s| s.to_owned()).enumerate() {
            write(&dest.join(format!("{i}.txt")), input);
        }
        Ok(())
    }
}

pub struct Library {
    /// The relative base library directory name
    pub name: String,
    /// Absolute path to the base library directory
    pub base_dir: PathBuf,
    /// Absolute path to the library config directory
    pub cfg_dir: PathBuf,
    /// Absolute path to the input directory
    pub input_dir: PathBuf,
    /// Absolute path to the output (target) directory
    pub output_dir: PathBuf,
}

impl Drop for Library {
    /// Remove the library folder and all its contents.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base_dir); // Ignore error when it did not exist
    }
}

impl Library {
    /// Create a new Library. The library will have a random name, and the base directory is
    /// relative to the current directory. This does not actually create any directories.
    pub fn new() -> Self {
        let name = Library::generate_name();
        let base_dir = env::current_dir().unwrap().join(&name);
        assert!(fs::metadata(&base_dir).is_err());
        let cfg_dir = base_dir.join(".tapeworm");

        Self {
            output_dir: cfg_dir.join("out"),
            input_dir: cfg_dir.join("in"),
            cfg_dir,
            base_dir,
            name,
        }
    }

    fn generate_name() -> String {
        let random = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        format!("tapeworm-test-{}", random)
    }

    /// Create just the base folder.
    /// With just the base folder, this library is invalid as a tapeworm library.
    pub fn create_base_folder(self) -> Self {
        fs::create_dir_all(&self.base_dir).unwrap();
        self
    }

    /// Create the config folder (implicitly creates the base folder).
    /// This makes the library a valid tapeworm library.
    pub fn create_cfg_folder(self) -> Self {
        fs::create_dir_all(&self.cfg_dir).unwrap();
        self
    }

    /// Create the input and output folders (implicitly creates the base and config folders).
    /// This makes the library a valid tapeworm library.
    pub fn create_in_out_folders(self) -> Self {
        fs::create_dir_all(&self.input_dir).unwrap();
        fs::create_dir_all(&self.output_dir).unwrap();
        self
    }

    /// Copy a test file to the library input folder. The input folder must be created first.
    ///
    /// # Parameters
    /// - `filename`: just the **name** of a file in the `resources/test` directory
    pub fn copy_to_input(&self, filename: &str) {
        let res_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("resources")
            .join("test");
        fs::copy(res_path.join(filename), self.input_dir.join(filename)).unwrap();
    }

    /// Returns the correct path str to use as the program's library argument.
    pub fn arg(&self) -> &str {
        self.base_dir.to_str().unwrap()
    }

    /// Returns a str to use as the program's INPUT_DIR argument.
    pub fn input_arg(&self) -> &str {
        self.input_dir.to_str().unwrap()
    }

    /// Returns a str to use as the program's TARGET_DIR argument.
    pub fn output_arg(&self) -> &str {
        self.output_dir.to_str().unwrap()
    }
}

/// # Parameters
/// - `args`: "command line" arguments
///
/// # Returns
/// - `Result<Config>`: the built Config or an error
pub fn build(mut args: Vec<&str>) -> Result<Config, Box<dyn std::error::Error>> {
    args.insert(0, "tapeworm");
    let args = args.into_iter().map(|s| String::from(s));
    Config::build(args)
}

/// Run the `config` and use `io::stdin` for reading any user input.
pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tapeworm::run(config, io::stdin().lock(), MockYtDlp {})
}

pub fn run_with<R: BufRead>(config: Config, reader: R) -> Result<(), Box<dyn std::error::Error>> {
    tapeworm::run(config, reader, MockYtDlp {})
}

/// # Returns
/// - `String`: the contents of the file at `path`
pub fn read(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

/// Write the `contents` to the file at `path`. If the file does not exist,
/// it is created; otherwise, it will be overwritten.
pub fn write(path: &PathBuf, contents: String) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(contents.as_bytes()).unwrap();
}
