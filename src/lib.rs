mod add;
mod download;
mod info;
mod organize;
mod scrape;
mod tag;
mod types;
mod util;

use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Default)]
pub struct Config {
    pub command: String,
    pub library: Option<String>,
    pub lib_desc: Option<String>,

    // Paths
    pub lib_path: Option<PathBuf>,
    pub lib_conf_path: Option<PathBuf>,
    pub input_path: Option<PathBuf>,
    pub yt_dlp_conf_path: Option<PathBuf>,

    // Add options
    pub terms: Option<Vec<String>>, // QUERY | URL...

    // Download options
    pub clear_input: bool,
    pub verbose: bool,

    // Tag options
    pub override_artist: bool,
    pub input_dir: Option<PathBuf>,

    // Deposit options
    /// If `None`, will cause `deposit` to simply drop files in the `target_dir`.
    /// Otherwise, it will be organized into the `target_dir` per below:
    /// - "A-Z": Sort into alphabetic subfolders, and possibly ARTIST and ALBUM subfolders
    pub organize: Option<String>,
    pub target_dir: Option<PathBuf>,

    // Process options
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

    /// Set up the library paths. If building for a command that is not "add",
    /// this will error if the library does not exist.
    fn parse_library(&mut self, library: Option<String>) -> types::UnitResult {
        if library.is_none() {
            return Err("Library not specified. See 'help'".into());
        }

        let library = library.unwrap();

        let lib_path = PathBuf::from(dirs::config_dir().unwrap())
            .join("tapeworm")
            .join(library.clone());

        if self.command != "add" && fs::metadata(&lib_path).is_err() {
            return Err(format!("Library not found: {}", lib_path.to_str().unwrap()).into());
        }

        let mut lib_conf_path = lib_path.join("lib");
        lib_conf_path.set_extension("conf");

        let mut input_path = lib_path.join("input");
        input_path.set_extension("txt");

        let mut yt_dlp_conf_path = lib_path.join("yt-dlp");
        yt_dlp_conf_path.set_extension("conf");

        self.library = Some(library);
        self.lib_path = Some(lib_path);
        self.lib_conf_path = Some(lib_conf_path);
        self.input_path = Some(input_path);
        self.yt_dlp_conf_path = Some(yt_dlp_conf_path);

        Ok(())
    }

    fn parse_terms(&mut self, mut args: impl Iterator<Item = String>) -> types::UnitResult {
        let mut terms = Vec::new();
        while let Some(arg) = args.next() {
            terms.push(arg);
        }

        if terms.is_empty() {
            Err("Provide search term(s) and/or URL(s). See 'help'".into())
        } else {
            self.terms = Some(terms);
            Ok(())
        }
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
                    "description" => self.lib_desc = Some(String::from(value)),
                    "verbose" => self.verbose = value.parse::<bool>()?,
                    // Download
                    "clear_input" => self.clear_input = value.parse::<bool>()?,
                    // Tag
                    "override_artist" => self.override_artist = value.parse::<bool>()?,
                    // Tag, Deposit
                    "input_dir" => self.input_dir = Some(PathBuf::from(value)),
                    // Deposit
                    "target_dir" => self.target_dir = Some(PathBuf::from(value)),
                    "organize" => self.organize = Some(String::from(value)),
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
                    'i' if self.command == "tag"
                        || self.command == "deposit"
                        || self.command == "process" =>
                    {
                        self.input_dir = args.next().map(PathBuf::from)
                    }
                    'd' if self.command == "deposit" || self.command == "process" => {
                        self.organize = args.next()
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
        if ["show", "add", "download", "tag", "deposit", "process"]
            .contains(&config.command.as_str())
        {
            config.parse_library(args.next())?;
        }

        // Parse extra options for commands that have them
        if config.command == "add" {
            config.parse_terms(args)?;
        } else if config.command == "show" {
            config.build_lib_conf_options()?; // just load the library settings
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
            let mut commands = Vec::with_capacity(steps.len());
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

pub fn run<R: BufRead>(config: Config, mut reader: R) -> types::UnitResult {
    for command in &config.steps()? {
        match command.as_str() {
            "list" => info::list()?,
            "show" => info::show(&config)?,
            "add" => add::add(&config)?,
            "download" => download::download(&config, &mut reader)?,
            "tag" => tag::tag(&config, &mut reader)?,
            "deposit" => organize::deposit(&config, &mut reader)?,
            _ => return Err("Unrecognized command. See 'help'".into()),
        }
    }

    Ok(())
}
