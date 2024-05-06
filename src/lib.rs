mod scrape;
mod tag;
mod types;
mod util;

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use url::Url;

#[derive(Default)]
pub struct Config {
    pub command: String,
    pub library: Option<String>,

    // Add
    pub terms: Option<Vec<String>>, // TERM... | URL...

    // Download options
    pub clear_input: bool,
    pub auto_scrape: bool,
    pub verbose: bool,

    // Paths
    pub lib_path: Option<PathBuf>,
    pub lib_conf_path: Option<PathBuf>,
    pub input_path: Option<PathBuf>,
    pub yt_dlp_conf_path: Option<PathBuf>,
    pub target_dir: Option<PathBuf>,

    // Tagging
    pub enable_tagging: bool,
    pub yt_dlp_output_dir: Option<PathBuf>,
}

impl Config {
    fn parse_command(command: Option<String>) -> types::StringBoolResult {
        if let Some(command) = command {
            return match command.as_str() {
                "help" | "h" | "-h" | "--help" => {
                    Config::help();
                    process::exit(0);
                }
                // Commands that require a library
                "add" | "download" => Ok((command, true)),
                // Commands that don't require a library
                "list" => Ok((command, false)),
                _ => Err("Unrecognized command. See 'help'".into()),
            };
        }

        Err("Command not specified. See 'help'".into())
    }

    fn parse_library(library: Option<String>) -> types::StringResult {
        if let Some(library) = library {
            Ok(library)
        } else {
            Err("Library not specified. See 'help'".into())
        }
    }

    fn parse_terms(&mut self, mut args: impl Iterator<Item = String>) -> types::UnitResult {
        let first = args.next();
        if first.is_none() {
            return Err("Provide either search term(s), or URL(s). See 'help'".into());
        }

        let mut terms: Vec<String> = Vec::new();

        let first = first.unwrap();

        if Url::parse(&first).is_ok() {
            terms.push(first);
            // If the first term parses as a URL, enforce that the rest are URLs too
            while let Some(arg) = args.next() {
                if Url::parse(&arg).is_err() {
                    return Err(format!("{} is not a URL. See 'help'", arg).into());
                }
                terms.push(arg);
            }
            self.terms = Some(terms);
        } else {
            terms.push(first);
            while let Some(arg) = args.next() {
                terms.push(arg);
            }
            // Otherwise, add all terms as a single query
            self.terms = Some(vec![terms.join(" ")]);
        }

        Ok(())
    }

    /// Override default options with options from lib.conf
    fn parse_lib_conf_options(&mut self) -> types::UnitResult {
        if fs::metadata(&self.lib_conf_path.clone().unwrap()).is_err() {
            return Ok(()); // Leave defaults if lib.conf doesn't exist
        }

        let options: Vec<String> = fs::read_to_string(&self.lib_conf_path.clone().unwrap())?
            .lines()
            .map(|line| line.trim().to_string())
            .collect();

        for line in options {
            if line.starts_with("#") {
                continue;
            }

            let option = line.split_once("=");
            if option.is_none() {
                return Err(format!("Invalid config line: {}", line).into());
            }

            let (key, value) = option.unwrap();
            match key.to_lowercase().as_str() {
                "auto_scrape" => self.auto_scrape = value.parse::<bool>()?,
                "clear_input" => self.clear_input = value.parse::<bool>()?,
                "enable_tagging" => self.enable_tagging = value.parse::<bool>()?,
                "target_dir" => self.target_dir = Some(PathBuf::from(value)),
                "verbose" => self.verbose = value.parse::<bool>()?,
                "yt_dlp_output_dir" => self.yt_dlp_output_dir = Some(PathBuf::from(value)),
                _ => return Err(format!("Unrecognized config option: {}", key).into()),
            }
        }

        Ok(())
    }

    /// Override default options with CLI options
    fn parse_cli_options(&mut self, mut args: impl Iterator<Item = String>) -> types::UnitResult {
        while let Some(arg) = args.next() {
            if !arg.starts_with('-') {
                break; // no (more) options
            }

            for s in arg[1..].chars() {
                match s {
                    'c' => self.clear_input = true,
                    'y' => self.auto_scrape = true,
                    'v' => self.verbose = true,
                    _ => return Err("Unrecognized option. See 'help'".into()),
                };
            }
        }

        Ok(())
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> types::ConfigResult {
        args.next(); // Consume program name

        let (command, require_library) = Config::parse_command(args.next())?;

        let mut config = if require_library {
            let library = Config::parse_library(args.next())?;

            let lib_path = PathBuf::from(dirs::config_dir().unwrap())
                .join("tapeworm")
                .join(library.clone());

            let mut lib_conf_path = lib_path.join("lib");
            lib_conf_path.set_extension("conf");

            let mut input_path = lib_path.join("input");
            input_path.set_extension("txt");

            let mut yt_dlp_conf_path = lib_path.join("yt-dlp");
            yt_dlp_conf_path.set_extension("conf");

            Config {
                command,
                library: Some(library),
                lib_path: Some(lib_path),
                lib_conf_path: Some(lib_conf_path),
                input_path: Some(input_path),
                yt_dlp_conf_path: Some(yt_dlp_conf_path),
                ..Default::default()
            }
        } else {
            Config {
                command,
                ..Default::default()
            }
        };

        match config.command.as_str() {
            "add" => config.parse_terms(args)?,
            "download" => {
                // Override defaults with lib.conf, then with CLI options
                config.parse_lib_conf_options()?;
                config.parse_cli_options(args)?;
            }
            "list" => {}
            _ => return Err("Unrecognized command. See 'help'".into()),
        };

        Ok(config)
    }

    fn help() {
        println!(
            "\
tapeworm - A scraper and downloader written in Rust

COMMANDS
    help
        Show this help message

    list
        List all libraries

    add LIBRARY URL [URL...]
        Add URLs to the library.
        The library is created if it does not exist

    add LIBRARY TERM [TERM...]
        Combine all terms into a single search query and add it to the library.
        The library is created if it does not exist.
        NB: when invoking 'download', a YouTube video will be found for the query

    download LIBRARY [OPTIONS]
        Given the inputs in ~/.config/tapeworm/LIBRARY/input.txt,
        scrape any queries and download all (scraped) URLs,
        using the config in ~/.config/tapeworm/LIBRARY/yt-dlp.conf

DOWNLOAD OPTIONS
    The options from ~/.config/tapeworm/LIBRARY/lib.conf are loaded first.
    Setting a CLI option will override its value in the lib.conf file, if present.

    -c      Clear the input file after scraping
    -v      Verbosely show what is being processed
    -y      Automatically select the best scraped link if any are found

EXAMPLE
    # Create the library by recording the first query
    tapeworm add LIBRARY the artist - a song  # records 'the artist - a song'
    # Add a URL
    tapeworm add LIBRARY https://youtube.com/watch?v=123
    # Scrape/download all
    tapeworm download LIBRARY
"
        );
    }

    /// Returns:
    /// - Some(true) if yt-dlp.conf exists, it will be used
    /// - Some(false) if the user wants to continue without yt-dlp.conf
    /// - None if the user wants to abort
    fn yt_dlp_conf_exists(&self) -> types::BoolResult {
        if fs::metadata(&self.yt_dlp_conf_path.clone().unwrap()).is_ok() {
            return Ok(Some(true));
        }

        println!(
            "Warning: {} not found
If you continue, yt-dlp will be invoked without any options, which will yield inconsistent results. Do you want to continue regardless? y/N",
            self.yt_dlp_conf_path.clone().unwrap().to_str().unwrap()
        );

        let input = util::input()?;
        if input.is_empty() || input.starts_with('n') {
            Ok(None)
        } else {
            Ok(Some(false))
        }
    }
}

/// Attempts to append all terms to the input file.
/// The library folder and input file are created if they do not exist.
fn add(config: &Config) -> types::UnitResult {
    if fs::metadata(&config.lib_path.clone().unwrap()).is_err() {
        fs::create_dir_all(&config.lib_path.clone().unwrap())?;
    }

    let mut input_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.input_path.clone().unwrap())?;

    let contents = format!("{}\n", config.terms.as_ref().unwrap().join("\n"));
    input_file.write_all(contents.as_bytes())?;

    Ok(())
}

/// Download URLs with yt-dlp
fn yt_dlp(config: &Config, use_conf: bool, urls: HashSet<String>) -> types::UnitResult {
    let mut command = Command::new("yt-dlp");
    if use_conf {
        command
            .arg("--config-location")
            .arg(&config.yt_dlp_conf_path.clone().unwrap());
    }
    urls.iter().for_each(|url| {
        command.arg(url);
    });
    command.stdout(Stdio::piped());

    let stdout = command.spawn()?.stdout.ok_or_else(|| {
        std::io::Error::new(ErrorKind::Other, "Could not capture standard output.")
    })?;

    BufReader::new(stdout)
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| println!("{}", line));

    Ok(())
}

fn download(config: &Config) -> types::UnitResult {
    if fs::metadata(&config.lib_path.clone().unwrap()).is_err() {
        return Err(format!(
            "Library not found: {}",
            config.lib_path.clone().unwrap().to_str().unwrap()
        )
        .into());
    }

    let use_yt_dlp_conf = if let Some(value) = config.yt_dlp_conf_exists()? {
        value
    } else {
        return Ok(()); // User wants to abort when config is not found
    };

    if fs::metadata(&config.input_path.clone().unwrap()).is_err() {
        return Err(format!(
            "Input file not found: {}",
            config.input_path.clone().unwrap().to_str().unwrap()
        )
        .into());
    }

    let inputs = fs::read_to_string(&config.input_path.clone().unwrap())?;
    if inputs.is_empty() {
        if config.verbose {
            println!("Nothing to download. Library is empty.");
        }
        return Ok(());
    }

    let (urls, queries): (Vec<_>, Vec<_>) = inputs
        .lines()
        .map(|s| s.to_string())
        .partition(|s| Url::parse(s).is_ok());

    let mut inputs: HashSet<String> = HashSet::new();
    inputs.extend(urls); // only keep unique URLs

    let total = queries.len();
    if total > 0 {
        let browser = headless_chrome::Browser::default().unwrap();
        let tab = browser.new_tab().unwrap();

        for (i, query) in queries.iter().enumerate() {
            let query = format!(
                "https://www.youtube.com/results?search_query={}",
                query.replace(" ", "+")
            );
            println!("Scraping {} of {}: {} ...", i + 1, total, query);

            let url = scrape::scrape_page(&config, &tab, query)?;
            if let Some(url) = url {
                inputs.insert(url);
            } // skip None
        }
    }

    if config.verbose {
        println!("Downloading {} URLs:", inputs.len());
        inputs.iter().for_each(|s| println!("  {}", s));
    }

    yt_dlp(&config, use_yt_dlp_conf, inputs)?;

    if config.clear_input {
        fs::write(&config.input_path.clone().unwrap(), "")?;
    }

    Ok(())
}

fn list() -> types::UnitResult {
    let conf_path = PathBuf::from(dirs::config_dir().unwrap()).join("tapeworm");
    let libraries = fs::read_dir(&conf_path);
    if libraries.is_err() {
        return Ok(()); // No need to fail when no libraries are present
    }

    for library in libraries.unwrap() {
        let library = library?;
        if library.file_type()?.is_dir() {
            println!("{}", library.file_name().to_str().unwrap());
        }
    }

    Ok(())
}

/// Attempt to move all downloaded (and processed) files in YT_DLP_OUTPUT_DIR to TARGET_DIR.
/// TARGET_DIR is created if not present.
/// Directories are not moved, only files.
/// If a file already exists in TARGET_DIR, it will be overwritten.
fn post_process(config: &Config) -> types::UnitResult {
    if config.target_dir.is_none() {
        return Ok(());
    } else if config.yt_dlp_output_dir.is_none() {
        return Err(
            "'YT_DLP_OUTPUT_DIR' must be set for moving downloads to 'TARGET_DIR'. See 'help'"
                .into(),
        );
    }

    let target_dir =
        PathBuf::from(config.lib_path.clone().unwrap()).join(config.target_dir.clone().unwrap());
    if fs::metadata(&target_dir).is_err() {
        fs::create_dir_all(&target_dir)?;
    }

    let downloads = PathBuf::from(config.lib_path.clone().unwrap())
        .join(config.yt_dlp_output_dir.clone().unwrap());
    for entry in fs::read_dir(downloads)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            continue;
        }

        fs::rename(entry.path(), target_dir.join(entry.file_name()))?;
        println!(
            "Moved {} to {}",
            entry.file_name().to_str().unwrap(),
            target_dir.display()
        );
    }

    Ok(())
}

pub fn run(config: Config) -> types::UnitResult {
    match config.command.as_str() {
        "add" => add(&config),
        "download" => {
            download(&config)?;
            tag::tag(&config)?;
            post_process(&config)
        }
        "list" => list(),
        _ => Ok(()),
    }
}
