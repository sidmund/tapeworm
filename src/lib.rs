// TODO ability to scrape spotify page for track title and artist name, and then find corresponding
// youtube urls for them

use std::error::Error;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::process;
// use std::process::Command;
use url::Url;

pub struct Config {
    pub mode: Option<&'static str>,
    pub input_file: Option<String>,
    pub remove_input_file: bool,
    pub terms: Option<Vec<String>>,
    pub output_file: Option<String>,
    pub extract_audio: bool,
    pub verbose: bool,
    pub interactive: bool,
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next(); // Consume program name

        // Setup defaults
        let mut config = Config {
            mode: None,
            input_file: None,
            remove_input_file: false,
            terms: None,
            output_file: None,
            extract_audio: false,
            verbose: false,
            interactive: true,
        };

        // Parse mode
        if let Some(mode) = args.next() {
            match mode.as_str() {
                "help" | "h" | "-h" | "--help" => {
                    Config::help();
                    process::exit(0);
                }
                "hook" => config.mode = Some("hook"),
                "suck" => config.mode = Some("suck"),
                _ => return Err("Unrecognized mode. See 'help'"),
            }
        } else {
            return Err("Mode not specified. See 'help'");
        }

        let mut terms: Vec<String> = Vec::new();

        // Parse options
        while let Some(arg) = args.next() {
            // No (more) options
            if !arg.starts_with('-') {
                terms.push(arg);
                break;
            }

            // Support combined options, e.g. -af
            for s in arg[1..].chars() {
                let mut matched = false;

                // Parse mode specific options
                if config.mode == Some("hook") {
                    matched = match s {
                        'o' => {
                            let output_file = args.next();
                            if output_file.is_none() {
                                return Err("Missing output file path. See 'help'");
                            }
                            config.output_file = Some(output_file.unwrap());
                            true
                        }
                        'y' => {
                            config.interactive = false;
                            true
                        }
                        _ => false,
                    }
                } else if config.mode == Some("suck") {
                    matched = match s {
                        'a' => {
                            config.extract_audio = true;
                            true
                        }
                        _ => false,
                    }
                }

                if matched {
                    continue;
                }

                // Fallback to parse general options
                match s {
                    'f' => {
                        let input_file = args.next();
                        if input_file.is_none() {
                            return Err("Missing file path. See 'help'");
                        }
                        config.input_file = Some(input_file.unwrap());
                    }
                    'd' => config.remove_input_file = true,
                    'v' => config.verbose = true,
                    _ => return Err("Unrecognized option. See 'help'"),
                }
            }
        }

        if config.input_file.is_some() {
            // Ignore terms/urls if the file is specified
            return Ok(config);
        }

        // Treat the remaining arguments as search term input
        while let Some(arg) = args.next() {
            terms.push(arg);
        }

        if terms.is_empty() {
            return Err("Provide either a file path or search terms. See 'help'");
        }

        config.terms = Some(terms);

        Ok(config)
    }

    fn help() {
        println!(
            "\
tapeworm - A scraper and downloader for YouTube written in Rust

DESCRIPTION
    tapeworm has two modes, inspired by the anatomy of its real-life eponym.

    The first mode, 'hook', scrapes YouTube with the given query to find a video URL.
    The user can select one of the found URLs.

    The second mode, 'suck', downloads given URLs as audio or video using yt-dlp.

USAGE
    tapeworm help
    tapeworm hook [OPTIONS] [TERM]...
    tapeworm suck [OPTIONS] [URL]...

    If TERM or URL is unspecified, the -f option must be set.
    TERM consists of space-separated terms, combined to form a single query.
    URL consists of space-separated URLs, treated as separate.

OPTIONS
    Any mode:
    -f FILE
        Read queries / URLs from the file, each line is treated as a separate query / URL.
        If set, TERM / URL is ignored

    -d
        If -f is set, this will remove (delete) the input file after processing.
        By default, the FILE is only read

    -v
        Verbosely show what is being processed

    Mode 'hook':
    -o FILE
        Write URLs to the file instead of stdout. If the file exists, it is overwritten

    -y
        Automatically select the best scraped link if any are found

    Mode 'suck':
    -a
        Download as audio, instead of video (the default)

EXAMPLES
    # Query for 'artist title' and return a relevant URL
    tapeworm hook artist title
    # Query each line in the file and return relevant URLs
    tapeworm hook -f queries.txt

    # Download the URL as video
    tapeworm suck https://www.youtube.com/watch?v=xxx
    # Download the URLs from the file as audio
    tapeworm suck -af urls.txt
"
        );
    }

    fn write_default_config_if_missing(self, filename: &str) -> Result<(), Box<dyn Error>> {
        let config_file_result = File::open(filename);
        if config_file_result.is_ok() {
            return Ok(());
        }

        match config_file_result.unwrap_err().kind() {
            ErrorKind::NotFound => {
                let mut file = File::create(filename)?;
                if filename == "tapeworm-audio.conf" {
                    file.write_all(b"# Default configuration for downloading audio\n")?;
                    file.write_all(b"# For options, see https://github.com/yt-dlp/yt-dlp\n\n")?;
                    file.write_all(b"-r 4M\n")?;
                    file.write_all(b"-i\n")?;
                    file.write_all(b"-x\n")?;
                    file.write_all(b"--audio-format mp3\n")?;
                    file.write_all(b"-f bestaudio\n")?;
                    file.write_all(b"--audio-quality 320k\n")?;
                    file.write_all(b"-o \"%(title)s.%(ext)s\"\n")?;
                } else {
                    file.write_all(b"# Default configuration for downloading video\n")?;
                    file.write_all(b"# For options, see https://github.com/yt-dlp/yt-dlp\n\n")?;
                    // TODO video defaults
                }
                Ok(())
            }
            other_error => Err(Box::new(io::Error::from(other_error))),
        }
    }

    /// Scrape YouTube with given inputs.
    /// Returns a list of URLs, one per input
    fn hook(&self, inputs: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
        if inputs.is_empty() {
            return Ok(inputs);
        }

        let inputs: Vec<String> = inputs
            .iter()
            .map(|line| line.replace(" ", "+").to_string())
            .collect();
        let total = inputs.len();

        let browser = headless_chrome::Browser::default().unwrap();
        let tab = browser.new_tab().unwrap();

        let mut urls = Vec::new();

        for (i, query) in inputs.iter().enumerate() {
            if self.verbose {
                println!("Scraping {} of {}: {} ...", i + 1, total, query);
            }

            let url = format!("https://www.youtube.com/results?search_query={}", query);
            tab.navigate_to(url.as_str()).unwrap();

            let mut results = Vec::new();

            let results_html = tab.wait_for_elements(".title-and-badge").unwrap();
            for result_html in results_html {
                let attributes = result_html
                    .wait_for_element("a")
                    .unwrap()
                    .get_attributes()
                    .unwrap()
                    .unwrap();

                if self.verbose {
                    println!("Found attributes: {}", attributes.join(" "));
                }

                let title = attributes.get(7).unwrap().clone();
                // Format: /watch?v=kOGx_qFC4aM&pp=ygUEcnVzdA%3D%3D
                let rel_url = attributes.get(9).unwrap();
                let url = format!(
                    "https://www.youtube.com{}",
                    rel_url.split("&").next().unwrap()
                );

                results.push((title, url));

                if !self.interactive {
                    // Assume the first url is the best matched one
                    break;
                }
            }

            if results.is_empty() {
                println!("No results found for '{}', skipping", query);
                continue;
            }

            if !self.interactive {
                // Assume the first url is the best matched one
                urls.push(results.get(0).unwrap().1.clone());
                continue;
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
            urls.push(results.get(selected).unwrap().1.clone());
        }

        Ok(urls)
    }

    /// Download given inputs as audio or video.
    ///
    /// Inputs may contain a mix of URLs and queries.
    /// Any queries are converted to URLs using hook().
    fn suck(self, inputs: Vec<String>) -> Result<(), Box<dyn Error>> {
        if inputs.is_empty() {
            return Ok(());
        }

        let (urls, queries): (Vec<_>, Vec<_>) = inputs.iter().partition(|s| Url::parse(s).is_ok());

        let inputs: Vec<String> = self
            .hook(queries.iter().map(|s| s.to_string()).collect())?
            .into_iter()
            .chain(urls.iter().map(|s| s.to_string()))
            .collect();

        let config_file = if self.extract_audio {
            "tapeworm-audio.conf"
        } else {
            "tapeworm-video.conf"
        };
        self.write_default_config_if_missing(config_file)?;

        println!("Downloading {} ...", inputs.join(" "));
        println!("Using config file: {}", config_file);

        // TODO download using yt-dlp
        // let output = Command::new("yt-dlp")
        //     .arg("--config-location")
        //     .arg(config_file)
        //     .arg(inputs.join(" "))
        //     .output()
        //     .expect("failed to execute yt-dlp");

        // println!("status: {}", output.status);
        // println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        // println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        // assert!(output.status.success());

        Ok(())
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let mut inputs = Vec::new();
    if let Some(input_file) = &config.input_file {
        let contents = fs::read_to_string(input_file)?;
        for line in contents.lines() {
            inputs.push(line.to_string());
        }
    } else if let Some(terms) = &config.terms {
        if Url::parse(&terms[0]).is_ok() {
            // Treat URL args as separate inputs
            inputs.extend(terms.iter().map(|s| s.to_string()));
        } else {
            // Treat term args as a single input
            inputs.push(terms.join(" "));
        }
    }

    match config.mode {
        Some("hook") => {
            let urls = config.hook(inputs)?;

            // Output
            if let Some(output_file) = &config.output_file {
                if config.verbose {
                    println!("Writing {} URLs to {} ...", urls.len(), output_file);
                }
                fs::write(output_file, urls.join("\n"))?;
            } else {
                urls.iter().for_each(|url| println!("{}", url));
            }

            // Cleanup
            if config.remove_input_file {
                if let Some(input_file) = config.input_file {
                    if config.verbose {
                        println!("Removing {} ...", input_file);
                    }
                    // TODO maybe removing is better, and we can add stuff to the input file by just
                    // recreating it, or maybe even using tapeworm, e.g. `tapeworm add URL`
                    // I like the idea of tapeworm managing the backlog
                    fs::remove_file(input_file)?;
                }
            }

            Ok(())
        }
        Some("suck") => config.suck(inputs),
        _ => Ok(()),
    }
}
