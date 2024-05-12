mod add;
mod download;
mod info;
mod organize;
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
    pub yt_dlp_output_dir: Option<PathBuf>,

    // Depositing
    pub deposit_az: bool,
    pub target_dir: Option<PathBuf>,

    // Processing
    pub steps: Option<Vec<String>>,
}

impl Config {
    fn parse_command(command: Option<String>) -> types::StringResult {
        if let Some(command) = command {
            match command.as_str() {
                "help" | "h" | "-h" | "--help" => {
                    info::help();
                    process::exit(0);
                }
                _ => Ok(command),
            }
        } else {
            Err("Command not specified. See 'help'".into())
        }
    }

    fn parse_library(library: Option<String>) -> types::StringResult {
        if let Some(library) = library {
            Ok(library)
        } else {
            Err("Library not specified. See 'help'".into())
        }
    }

    fn setup_library_paths(&mut self) {
        let lib_path = PathBuf::from(dirs::config_dir().unwrap())
            .join("tapeworm")
            .join(self.library.clone().unwrap());

        let mut lib_conf_path = lib_path.join("lib");
        lib_conf_path.set_extension("conf");

        let mut input_path = lib_path.join("input");
        input_path.set_extension("txt");

        let mut yt_dlp_conf_path = lib_path.join("yt-dlp");
        yt_dlp_conf_path.set_extension("conf");

        self.lib_path = Some(lib_path);
        self.lib_conf_path = Some(lib_conf_path);
        self.input_path = Some(input_path);
        self.yt_dlp_conf_path = Some(yt_dlp_conf_path);
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

    /// Attempt to read in options from lib.conf if it exists.
    /// For any option that is not present in the file, the default will be kept.
    ///
    /// # Errors
    /// - If a line does not follow the `option=value` format
    /// - If an option is not recognized
    fn build_lib_conf_options(&mut self) -> types::UnitResult {
        let contents = fs::read_to_string(&self.lib_conf_path.clone().unwrap());
        if contents.is_err() {
            return Ok(()); // Leave defaults when file not present
        }

        for line in contents.unwrap().lines().map(|l| l.trim()) {
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            if let Some((key, value)) = line.split_once("=") {
                match key.to_lowercase().as_str() {
                    // General
                    "verbose" => self.verbose = value.parse::<bool>()?,
                    // Download
                    "clear_input" => self.clear_input = value.parse::<bool>()?,
                    // Tag, Deposit
                    "yt_dlp_output_dir" => self.yt_dlp_output_dir = Some(PathBuf::from(value)),
                    // Deposit
                    "target_dir" => self.target_dir = Some(PathBuf::from(value)),
                    "deposit_az" => self.deposit_az = value.parse::<bool>()?,
                    // Process
                    "steps" => self.steps = Some(value.split(',').map(String::from).collect()),
                    _ => return Err(format!("Invalid config option: {}", key).into()),
                }
            } else {
                return Err(format!("Invalid config line: {}", line).into());
            }
        }

        Ok(())
    }

    /// Attempts to override options with CLI options.
    ///
    /// # Errors
    /// - If an option is not recognized for the Config's command
    fn parse_cli_options(&mut self, mut args: impl Iterator<Item = String>) -> types::UnitResult {
        while let Some(arg) = args.next() {
            if !arg.starts_with('-') {
                break; // no (more) options
            }

            for c in arg[1..].chars() {
                match c {
                    'v' => self.verbose = true,
                    'c' if self.command == "download" || self.command == "process" => {
                        self.clear_input = true
                    }
                    'y' if self.command == "tag"
                        || self.command == "deposit"
                        || self.command == "process" =>
                    {
                        self.yt_dlp_output_dir = args.next().map(PathBuf::from)
                    }
                    'd' if self.command == "deposit" || self.command == "process" => {
                        self.deposit_az = true
                    }
                    'o' if self.command == "deposit" || self.command == "process" => {
                        self.target_dir = args.next().map(PathBuf::from)
                    }
                    's' if self.command == "process" => {
                        self.steps =
                            Some(args.next().unwrap().split(',').map(String::from).collect())
                    }
                    _ => {
                        return Err(format!(
                            "Unrecognized option '{}' for command '{}'. See 'help'",
                            c, self.command
                        )
                        .into())
                    }
                }
            }
        }

        Ok(())
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> types::ConfigResult {
        args.next(); // Consume program name

        let command = Config::parse_command(args.next())?;

        let mut config = Config {
            command,
            ..Default::default()
        };

        // Commands that require a library
        if ["show", "download", "tag", "deposit", "process"].contains(&config.command.as_str()) {
            config.library = Some(Config::parse_library(args.next())?);
            config.setup_library_paths();
        }

        // Parse extra options for commands that have them
        if config.command == "add" {
            config.parse_terms(args)?;
        } else if ["download", "tag", "deposit", "process"].contains(&config.command.as_str()) {
            // Commands that use options from lib.conf / CLI
            config.build_lib_conf_options()?; // override defaults with lib.conf
            config.parse_cli_options(args)?; // override defaults/lib.conf with CLI
        }

        Ok(config)
    }

    fn steps(&self) -> Result<Vec<&String>, Box<dyn std::error::Error>> {
        if self.command.as_str() != "process" {
            return Ok(vec![&self.command]);
        }

        if let Some(steps) = &self.steps {
            let mut commands = Vec::new();
            for step in steps {
                if step == "add" || step == "process" {
                    return Err(format!(
                        "Command '{}' not supported as a processing step. See 'help'",
                        step
                    )
                    .into());
                }
                commands.push(step);
            }
            return Ok(commands);
        }

        Err("No processing steps specified. See 'help'".into())
    }
}

pub fn run(config: Config) -> types::UnitResult {
    for command in &config.steps()? {
        match command.as_str() {
            "list" => info::list()?,
            "show" => info::show(&config)?,
            "add" => add::add(&config)?,
            "download" => download::download(&config)?,
            "tag" => tag::tag(&config)?,
            "deposit" => organize::deposit(&config)?,
            _ => return Err("Unrecognized command. See 'help'".into()),
        }
    }

    Ok(())
}
