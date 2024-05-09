mod add;
mod download;
mod info;
mod scrape;
mod tag;
mod types;
mod util;

use std::fs;
use std::path::PathBuf;
use std::process;
use url::Url;

#[derive(Default)]
pub struct Config {
    pub command: String,
    pub library: Option<String>,

    // Add
    pub terms: Option<Vec<String>>, // QUERY | URL...

    // Download options
    pub clear_input: bool,
    pub verbose: bool,

    // Paths
    pub lib_path: Option<PathBuf>,
    pub lib_conf_path: Option<PathBuf>,
    pub input_path: Option<PathBuf>,
    pub yt_dlp_conf_path: Option<PathBuf>,

    // Tagging
    pub enable_tagging: bool,
    pub yt_dlp_output_dir: Option<PathBuf>,

    // Depositing
    pub deposit_az: bool,
    pub target_dir: Option<PathBuf>,
}

impl Config {
    fn parse_command(command: Option<String>) -> types::StringBoolResult {
        if let Some(command) = command {
            return match command.as_str() {
                "help" | "h" | "-h" | "--help" => {
                    info::help();
                    process::exit(0);
                }
                // Commands that require a library
                "show" | "add" | "download" => Ok((command, true)),
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
            let query = format!("ytsearch:\"{}\"", terms.join(" "));
            self.terms = Some(vec![query]);
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
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            let option = line.split_once("=");
            if option.is_none() {
                return Err(format!("Invalid config line: {}", line).into());
            }

            let (key, value) = option.unwrap();
            match key.to_lowercase().as_str() {
                "clear_input" => self.clear_input = value.parse::<bool>()?,
                "deposit_az" => self.deposit_az = value.parse::<bool>()?,
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
                    'd' => self.deposit_az = true,
                    'o' => self.target_dir = args.next().map(PathBuf::from),
                    't' => self.enable_tagging = true,
                    'v' => self.verbose = true,
                    'y' => self.yt_dlp_output_dir = args.next().map(PathBuf::from),
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
            "show" => {}
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

    /// Returns:
    /// - Some(true) if yt-dlp.conf exists, it will be used
    /// - Some(false) if the user wants to continue without yt-dlp.conf
    /// - None if the user wants to abort
    fn yt_dlp_conf_exists(&self) -> types::OptionBoolResult {
        if fs::metadata(&self.yt_dlp_conf_path.clone().unwrap()).is_ok() {
            return Ok(Some(true));
        }

        println!(
            "Warning: {} not found
If you continue, yt-dlp will be invoked without any options, which will yield inconsistent results.",
            self.yt_dlp_conf_path.clone().unwrap().to_str().unwrap()
        );

        if util::confirm("Do you want to continue regardless?", false)? {
            Ok(Some(false))
        } else {
            Ok(None)
        }
    }
}

pub fn run(config: Config) -> types::UnitResult {
    match config.command.as_str() {
        "show" => info::show(&config),
        "add" => add::add(&config),
        "download" => download::download(&config),
        "list" => info::list(),
        _ => Ok(()),
    }
}
