use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::process::{self, Command};
use url::Url;

pub struct Config {
    pub command: String,
    pub library: String,

    // Add
    pub terms: Option<Vec<String>>, // FILE... | TERM... | URL...

    // CLI download options
    pub download_options: HashMap<String, bool>,
    // Runtime download options
    pub clear_input: bool,
    pub auto_scrape: bool,
    pub verbose: bool,
}

impl Config {
    fn parse_command(command: Option<String>) -> Result<String, String> {
        if let Some(command) = command {
            return match command.as_str() {
                "help" | "h" | "-h" | "--help" => {
                    Config::help();
                    process::exit(0);
                }
                "add" | "download" => Ok(command),
                _ => Err(String::from("Unrecognized command. See 'help'")),
            };
        }

        Err(String::from("Command not specified. See 'help'"))
    }

    fn parse_library(library: Option<String>) -> Result<String, String> {
        if let Some(library) = library {
            Ok(library)
        } else {
            Err(String::from("Library not specified. See 'help'"))
        }
    }

    fn parse_terms(&mut self, mut args: impl Iterator<Item = String>) -> Result<(), String> {
        let first = args.next();
        if first.is_none() {
            return Err(String::from(
                "Provide either input file(s), or search term(s), or URL(s). See 'help'",
            ));
        }

        let mut terms: Vec<String> = Vec::new();

        let first = first.unwrap();

        if Url::parse(&first).is_ok() {
            terms.push(first);
            // If the first term parses as a URL, enforce that the rest are URLs too
            while let Some(arg) = args.next() {
                if Url::parse(&arg).is_err() {
                    return Err(String::from(format!("{} is not a URL. See 'help'", arg)));
                }
                terms.push(arg);
            }
            self.terms = Some(terms);
        } else if fs::metadata(&first).is_ok() {
            terms.push(first);
            // If the first term is an existent file path, enforce that the rest are too
            while let Some(arg) = args.next() {
                if fs::metadata(&arg).is_err() {
                    return Err(String::from(format!("{} is not a file. See 'help'", arg)));
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

    fn parse_options(&mut self, mut args: impl Iterator<Item = String>) -> Result<(), String> {
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
                    _ => return Err(String::from("Unrecognized option. See 'help'")),
                };
            }
        }

        Ok(())
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, String> {
        args.next(); // Consume program name

        let command = Config::parse_command(args.next())?;
        let library = Config::parse_library(args.next())?;

        // Setup defaults
        let mut config = Config {
            command,
            library,
            terms: None,
            download_options: HashMap::new(),
            clear_input: false,
            auto_scrape: false,
            verbose: false,
        };

        match config.command.as_str() {
            "add" => config.parse_terms(args)?,
            "download" => config.parse_options(args)?,
            _ => return Err(String::from("Unrecognized command. See 'help'")),
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

    tapeworm add LIBRARY [FILE... | TERM... | URL...]
        Add a term or URL to the library. If LIBRARY doesn't exist, it is created.
        Each FILE may contain queries or URLs, separated on newlines;
        TERM consists of space-separated terms, combined to form a single query;
        URL consists of space-separated URLs, treated as separate

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
    tapeworm LIBRARY the artist - a song  # records 'the artist - a song'
    # Add a URL
    tapeworm LIBRARY https://youtube.com/watch?v=123
    # Scrape/download all
    tapeworm LIBRARY
"
        );
    }

    fn get_library_path(&self) -> String {
        let library_path = format!(
            "{}/.config/tapeworm/{}/",
            std::env::home_dir().unwrap().to_str().unwrap(),
            self.library
        );
        println!("Using library path: {}", library_path);

        library_path
    }

    fn library_exists(&self) -> Result<String, Box<dyn Error>> {
        let library_path = self.get_library_path();

        if fs::metadata(&library_path).is_err() {
            Err(format!("Library not found: {}", library_path).into())
        } else {
            Ok(library_path)
        }
    }

    fn get_download_options(&mut self, lib_conf_path: String) -> Result<(), Box<dyn Error>> {
        println!("Using config path: {}", lib_conf_path);

        if fs::metadata(&lib_conf_path).is_ok() {
            // Override defaults with config file
            let lib_conf = fs::read_to_string(lib_conf_path)?;
            let options: Vec<String> = lib_conf
                .lines()
                .map(|line| line.trim().to_lowercase())
                .collect();
            for line in options {
                let parts: Vec<String> = line.split("=").map(|s| s.to_string()).collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid config line: {}", line).into());
                }

                match parts[0].as_str() {
                    "clear_input" => {
                        self.clear_input = parts[1].parse::<bool>()?;
                    }
                    "auto_scrape" => {
                        self.auto_scrape = parts[1].parse::<bool>()?;
                    }
                    "verbose" => {
                        self.verbose = parts[1].parse::<bool>()?;
                    }
                    _ => return Err(format!("Invalid config option: {}", parts[0]).into()),
                }
            }
        }

        // Override defaults with CLI options
        if self.download_options.contains_key("clear_input") {
            self.clear_input = *self.download_options.get("clear_input").unwrap();
        }
        if self.download_options.contains_key("auto_scrape") {
            self.auto_scrape = *self.download_options.get("auto_scrape").unwrap();
        }
        if self.download_options.contains_key("verbose") {
            self.verbose = *self.download_options.get("verbose").unwrap();
        }

        Ok(())
    }

    fn yt_dlp_conf_exists(yt_dlp_conf_path: String) -> Result<Option<bool>, Box<dyn Error>> {
        println!("Using yt-dlp path: {}", yt_dlp_conf_path);

        if fs::metadata(&yt_dlp_conf_path).is_err() {
            println!(
            "yt-dlp config not found at {}
If you continue, yt-dlp will be invoked without any options, which will yield inconsistent results. Do you want to continue regardless? y/N",
            yt_dlp_conf_path
        );

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input = input.trim().to_lowercase();
            if input.is_empty() || input.starts_with('n') {
                return Ok(None);
            }

            Ok(Some(false))
        } else {
            Ok(Some(true))
        }
    }

    fn input_file_exists(input_path: &String) -> Result<(), Box<dyn Error>> {
        println!("Using input path: {}", input_path);

        if fs::metadata(input_path).is_err() {
            Err(format!("Input file not found: {}", input_path).into())
        } else {
            Ok(())
        }
    }
}

fn scrape_page(
    config: &Config,
    tab: &headless_chrome::Tab,
    page: String,
) -> Result<Option<String>, Box<dyn Error>> {
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
        return Ok(Some(results.get(0).unwrap().1.clone()));
    }

    // Prompt user to select a result
    let selected = loop {
        println!("Select a result:");
        for (i, (title, url)) in results.iter().enumerate() {
            println!("  {}. {} | {}", i + 1, title, url);
        }

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let index = input.trim().to_string().parse::<usize>();
        if let Ok(index) = index {
            if index > 0 && index <= results.len() {
                break index - 1;
            }
        }
        println!("Invalid input, please try again");
    };
    Ok(Some(results.get(selected).unwrap().1.clone()))
}

/// Returns a list of URLs, one per input query
fn scrape(config: &Config, queries: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
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
        if config.verbose {
            println!("Scraping {} of {}: {}", i + 1, total, query);
        }

        let url = scrape_page(&config, &tab, query)?;
        if let Some(url) = url {
            urls.push(url);
        } // skip None
    }

    Ok(urls)
}

fn add(config: Config) -> Result<(), Box<dyn Error>> {
    // Create library folder if not present
    let library_path = config.get_library_path();
    if fs::metadata(&library_path).is_err() {
        fs::create_dir_all(&library_path)?;
    }

    // Create input file if not present
    let input_path = format!("{}/input.txt", library_path);
    println!("Using input path: {}", input_path);
    let mut input_file = File::open(&input_path);
    if input_file.is_err() {
        input_file = Ok(File::create(&input_path)?);
    }

    // Write all terms to file
    // TODO if they are files, write their contents instead
    // TODO does this append?
    input_file
        .unwrap()
        .write_all(config.terms.unwrap().join("\n").as_bytes())?;

    Ok(())
}

fn download(mut config: Config) -> Result<(), Box<dyn Error>> {
    let library_path = config.library_exists()?;

    let lib_conf_path = format!("{}/lib.conf", library_path);
    config.get_download_options(lib_conf_path)?;

    let yt_dlp_conf_path = format!("{}/yt-dlp.conf", library_path);
    let use_yt_dlp_conf = if let Some(value) = Config::yt_dlp_conf_exists(yt_dlp_conf_path)? {
        value
    } else {
        return Ok(()); // User wants to abort when config is not found
    };

    let input_path = format!("{}/input.txt", library_path);
    Config::input_file_exists(&input_path)?;

    let inputs = fs::read_to_string(&input_path)?;
    if inputs.is_empty() {
        println!("Nothing to download. Library is empty.");
        return Ok(());
    }

    let inputs: Vec<String> = fs::read_to_string(input_path.clone())?
        .lines()
        .map(|s| s.to_string())
        .collect();

    let (urls, queries): (Vec<_>, Vec<_>) = inputs.iter().partition(|s| Url::parse(s).is_ok());

    let scrape_inputs = queries.iter().map(|s| s.to_string()).collect();
    let scraped = scrape(&config, scrape_inputs)?;
    let inputs: Vec<String> = scraped
        .into_iter()
        .chain(urls.iter().map(|s| s.to_string()))
        .collect();

    println!("Downloading {} ...", inputs.join(" "));

    // TODO download using yt-dlp
    let output = if use_yt_dlp_conf {
        Command::new("echo")
            .arg("hello")
            .output()
            .expect("failed to execute yt-dlp")
        // Command::new("yt-dlp")
        //     .arg("--config-location")
        //     .arg(config_file)
        //     .arg(inputs.join(" "))
    } else {
        Command::new("echo")
            .arg("hello")
            .output()
            .expect("failed to execute yt-dlp")
        // Command::new("yt-dlp")
        //     .arg(inputs.join(" "))
    };

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());

    if config.clear_input {
        // TODO does this overwrite?
        File::create(input_path)?;
    }

    Ok(())
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    match config.command.as_str() {
        "add" => add(config),
        "download" => download(config),
        _ => Ok(()),
    }
}
