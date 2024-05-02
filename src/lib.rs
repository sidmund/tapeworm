use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use url::Url;

type ConfigResult = Result<Config, Box<dyn std::error::Error>>;
type UnitResult = Result<(), Box<dyn std::error::Error>>;
type BoolResult = Result<Option<bool>, Box<dyn std::error::Error>>;
type StringResult = Result<String, Box<dyn std::error::Error>>;
type StringOptionResult = Result<Option<String>, Box<dyn std::error::Error>>;
type StringVecResult = Result<Vec<String>, Box<dyn std::error::Error>>;

pub struct Config {
    pub command: String,
    pub library: String,

    // Add
    pub terms: Option<Vec<String>>, // TERM... | URL...

    // CLI download options
    pub download_options: HashMap<String, bool>,
    // Runtime download options
    pub clear_input: bool,
    pub auto_scrape: bool,
    pub verbose: bool,

    // Paths
    pub lib_path: PathBuf,
    pub lib_conf_path: PathBuf,
    pub input_path: PathBuf,
    pub yt_dlp_conf_path: PathBuf,
    pub target_dir: Option<PathBuf>,

    // Tagging
    pub enable_tagging: bool,
    pub yt_dlp_output_dir: Option<PathBuf>,
}

impl Config {
    fn parse_command(command: Option<String>) -> StringResult {
        if let Some(command) = command {
            return match command.as_str() {
                "help" | "h" | "-h" | "--help" => {
                    Config::help();
                    process::exit(0);
                }
                "add" | "download" => Ok(command),
                _ => Err("Unrecognized command. See 'help'".into()),
            };
        }

        Err("Command not specified. See 'help'".into())
    }

    fn parse_library(library: Option<String>) -> StringResult {
        if let Some(library) = library {
            Ok(library)
        } else {
            Err("Library not specified. See 'help'".into())
        }
    }

    fn parse_terms(&mut self, mut args: impl Iterator<Item = String>) -> UnitResult {
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

    fn parse_options(&mut self, mut args: impl Iterator<Item = String>) -> UnitResult {
        while let Some(arg) = args.next() {
            // No (more) options
            if !arg.starts_with('-') {
                break;
            }

            // Support combined options, e.g. -vy
            for s in arg[1..].chars() {
                match s {
                    'c' => self
                        .download_options
                        .insert(String::from("clear_input"), true),
                    'y' => self
                        .download_options
                        .insert(String::from("auto_scrape"), true),
                    'v' => self.download_options.insert(String::from("verbose"), true),
                    _ => return Err("Unrecognized option. See 'help'".into()),
                };
            }
        }

        Ok(())
    }

    fn get_download_options(&mut self) -> UnitResult {
        if fs::metadata(&self.lib_conf_path).is_ok() {
            // Override defaults with config file
            let lib_conf = fs::read_to_string(&self.lib_conf_path)?;
            let options: Vec<String> = lib_conf
                .lines()
                .map(|line| line.trim().to_string())
                .collect();
            for line in options {
                let parts: Vec<String> = line.split("=").map(|s| s.to_string()).collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid config line: {}", line).into());
                }

                match parts[0].to_lowercase().as_str() {
                    "auto_scrape" => {
                        self.auto_scrape = parts[1].parse::<bool>()?;
                    }
                    "clear_input" => {
                        self.clear_input = parts[1].parse::<bool>()?;
                    }
                    "enable_tagging" => {
                        self.enable_tagging = parts[1].parse::<bool>()?;
                    }
                    "target_dir" => {
                        self.target_dir = Some(PathBuf::from(parts[1].clone()));
                    }
                    "verbose" => {
                        self.verbose = parts[1].parse::<bool>()?;
                    }
                    "yt_dlp_output_dir" => {
                        self.yt_dlp_output_dir = Some(PathBuf::from(parts[1].clone()));
                    }
                    _ => return Err(format!("Invalid config option: {}", parts[0]).into()),
                }
            }
        }

        // Override defaults with CLI options
        if self.download_options.contains_key("auto_scrape") {
            self.auto_scrape = *self.download_options.get("auto_scrape").unwrap();
        }
        if self.download_options.contains_key("clear_input") {
            self.clear_input = *self.download_options.get("clear_input").unwrap();
        }
        if self.download_options.contains_key("verbose") {
            self.verbose = *self.download_options.get("verbose").unwrap();
        }

        Ok(())
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> ConfigResult {
        args.next(); // Consume program name

        let command = Config::parse_command(args.next())?;
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

        println!("Using library path: {:?}", lib_path);
        println!("Using input path  : {:?}", input_path);
        println!("Using config path : {:?}", lib_conf_path);
        println!("Using yt-dlp path : {:?}", yt_dlp_conf_path);

        // Setup defaults
        let mut config = Config {
            command,
            library,
            terms: None,
            download_options: HashMap::new(),
            clear_input: false,
            auto_scrape: false,
            verbose: false,
            lib_path,
            lib_conf_path,
            input_path,
            yt_dlp_conf_path,
            target_dir: None,
            enable_tagging: false,
            yt_dlp_output_dir: None,
        };

        match config.command.as_str() {
            "add" => config.parse_terms(args)?,
            "download" => {
                config.parse_options(args)?;
                config.get_download_options()?;
            }
            _ => return Err("Unrecognized command. See 'help'".into()),
        };

        Ok(config)
    }

    fn help() {
        println!(
            "\
tapeworm - A scraper and downloader written in Rust

COMMANDS
    tapeworm help
        Show this help message

    tapeworm add LIBRARY [TERM... | URL...]
        Add a term or URL to the library. If LIBRARY doesn't exist, it is created.
        TERM consists of space-separated terms, combined to form a single query;
        URL consists of space-separated URLs, treated as separate inputs

    tapeworm download LIBRARY [OPTIONS]
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

    fn yt_dlp_conf_exists(&self) -> BoolResult {
        if fs::metadata(&self.yt_dlp_conf_path).is_err() {
            println!(
            "Warning: {} not found
If you continue, yt-dlp will be invoked without any options, which will yield inconsistent results. Do you want to continue regardless? y/N",
            self.yt_dlp_conf_path.to_str().unwrap()
        );

            let input = input()?;
            if input.is_empty() || input.starts_with('n') {
                return Ok(None);
            }

            Ok(Some(false))
        } else {
            Ok(Some(true))
        }
    }
}

fn scrape_page(config: &Config, tab: &headless_chrome::Tab, page: String) -> StringOptionResult {
    tab.navigate_to(page.as_str())?;

    let mut results = Vec::new();
    for result_html in tab.wait_for_elements(".title-and-badge")? {
        let attributes = result_html
            .wait_for_element("a")?
            .get_attributes()?
            .unwrap();

        if config.verbose {
            println!("Found attributes: {}", attributes.join(" "));
        }

        let title = attributes.get(7).unwrap().clone();
        // Format: /watch?v=VIDEO_ID&OTHER_ARGS
        let rel_url = attributes.get(9).unwrap();
        let url = format!(
            "https://www.youtube.com{}",
            rel_url.split("&").next().unwrap()
        );

        results.push((title, url));
        if config.auto_scrape {
            // Assume the first url is the best matched one
            break;
        }
    }

    if results.is_empty() {
        println!("No results found for '{}', skipping", page);
        return Ok(None);
    }

    if config.auto_scrape {
        // Assume the first url is the best matched one
        let url = results.get(0).unwrap().1.clone();
        println!("Found: {}", url);
        return Ok(Some(url));
    }

    // Prompt user to select a result
    let selected = loop {
        println!("Select a result:");
        for (i, (title, url)) in results.iter().enumerate() {
            println!("  {}. {} | {}", i + 1, title, url);
        }

        let index = input()?.parse::<usize>();
        if index.as_ref().is_ok_and(|i| *i > 0 && *i <= results.len()) {
            break index.unwrap() - 1;
        }

        println!("Invalid input, please try again");
    };
    Ok(Some(results.get(selected).unwrap().1.clone()))
}

fn input() -> StringResult {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase())
}

/// Returns a list of URLs, one per input query
fn scrape(config: &Config, queries: Vec<String>) -> StringVecResult {
    if queries.is_empty() {
        return Ok(queries);
    }

    let queries: Vec<String> = queries
        .iter()
        .map(|line| line.replace(" ", "+").to_string())
        .collect();
    let total = queries.len();

    let browser = headless_chrome::Browser::default().unwrap();
    let tab = browser.new_tab().unwrap();

    let mut urls = Vec::new();

    for (i, query) in queries.iter().enumerate() {
        let query = format!("https://www.youtube.com/results?search_query={}", query);
        println!("Scraping {} of {}: {} ...", i + 1, total, query);

        let url = scrape_page(&config, &tab, query)?;
        if let Some(url) = url {
            urls.push(url);
        } // skip None
    }

    Ok(urls)
}

/// Attempts to append all terms to the input file.
/// The library folder and input file are created if they do not exist.
fn add(config: &Config) -> UnitResult {
    if fs::metadata(&config.lib_path).is_err() {
        fs::create_dir_all(&config.lib_path)?;
    }

    let mut input_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.input_path)?;

    let contents = format!("{}\n", config.terms.as_ref().unwrap().join("\n"));
    input_file.write_all(contents.as_bytes())?;

    Ok(())
}

fn download(config: &Config) -> UnitResult {
    if fs::metadata(&config.lib_path).is_err() {
        return Err(format!("Library not found: {}", config.lib_path.to_str().unwrap()).into());
    }

    let use_yt_dlp_conf = if let Some(value) = config.yt_dlp_conf_exists()? {
        value
    } else {
        return Ok(()); // User wants to abort when config is not found
    };

    if fs::metadata(&config.input_path).is_err() {
        return Err(format!(
            "Input file not found: {}",
            config.input_path.to_str().unwrap()
        )
        .into());
    }

    let inputs = fs::read_to_string(&config.input_path)?;
    if inputs.is_empty() {
        if config.verbose {
            println!("Nothing to download. Library is empty.");
        }
        return Ok(());
    }

    let inputs: Vec<String> = fs::read_to_string(config.input_path.clone())?
        .lines()
        .map(|s| s.to_string())
        .collect();

    let (urls, queries): (Vec<_>, Vec<_>) = inputs.iter().partition(|s| Url::parse(s).is_ok());

    let scrape_inputs = queries.iter().map(|s| s.to_string()).collect();

    // Only keep unique URLs
    let mut inputs: HashSet<String> = HashSet::new();
    inputs.extend(scrape(&config, scrape_inputs)?);
    inputs.extend(urls.iter().map(|s| s.to_string()));

    if config.verbose {
        println!("Downloading {} URLs:", inputs.len());
        for input in &inputs {
            println!("  {}", input);
        }
    }

    let inputs: Vec<String> = inputs.iter().map(|s| s.to_owned()).collect();
    let inputs = inputs.join(" ");

    // Download with yt-dlp
    let stdout = if use_yt_dlp_conf {
        Command::new("yt-dlp")
            .arg("--config-location")
            .arg(&config.yt_dlp_conf_path)
            .arg(inputs)
            .stdout(Stdio::piped())
            .spawn()?
            .stdout
            .ok_or_else(|| {
                std::io::Error::new(ErrorKind::Other, "Could not capture standard output.")
            })?
    } else {
        Command::new("yt-dlp")
            .arg(inputs)
            .stdout(Stdio::piped())
            .spawn()?
            .stdout
            .ok_or_else(|| {
                std::io::Error::new(ErrorKind::Other, "Could not capture standard output.")
            })?
    };
    let reader = BufReader::new(stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| println!("{}", line));

    if config.clear_input {
        fs::write(&config.input_path, "")?;
    }

    Ok(())
}

fn tag(config: &Config) -> UnitResult {
    if !config.enable_tagging {
        return Ok(());
    } else if config.yt_dlp_output_dir.is_none() {
        return Err("'YT_DLP_OUTPUT_DIR' must be set when tagging is enabled. See 'help'".into());
    }

    // yt_dlp_output_dir is appended to lib_path (if relative),
    // otherwise it will replace it (absolute)
    let downloads =
        PathBuf::from(config.lib_path.clone()).join(config.yt_dlp_output_dir.clone().unwrap());
    for entry in fs::read_dir(downloads)? {
        let entry = entry?;
        println!("{}", entry.file_name().to_str().unwrap());
    }

    Ok(())
}

/// Attempt to move all downloaded (and processed) files in YT_DLP_OUTPUT_DIR to TARGET_DIR.
/// TARGET_DIR is created if not present.
/// Directories are not moved, only files.
/// If a file already exists in TARGET_DIR, it will be overwritten.
fn post_process(config: &Config) -> UnitResult {
    if config.target_dir.is_none() {
        return Ok(());
    } else if config.yt_dlp_output_dir.is_none() {
        return Err(
            "'YT_DLP_OUTPUT_DIR' must be set for moving downloads to 'TARGET_DIR'. See 'help'"
                .into(),
        );
    }

    let target_dir =
        PathBuf::from(config.lib_path.clone()).join(config.target_dir.clone().unwrap());
    if fs::metadata(&target_dir).is_err() {
        fs::create_dir_all(&target_dir)?;
    }

    let downloads =
        PathBuf::from(config.lib_path.clone()).join(config.yt_dlp_output_dir.clone().unwrap());
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

pub fn run(config: Config) -> UnitResult {
    match config.command.as_str() {
        "add" => add(&config),
        "download" => {
            download(&config)?;
            tag(&config)?;
            post_process(&config)
        }
        _ => Ok(()),
    }
}
