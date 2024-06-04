mod add;
mod deposit;
mod download;
mod editor;
mod info;
mod scrape;
mod tag;
mod types;
mod util;

use crate::deposit::DepositMode;
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;

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
    pub auto_download: bool,
    pub verbose: bool,

    // Tag options
    pub override_artist: bool,
    pub title_template: String,
    pub filename_template: String,
    pub input_dir: Option<PathBuf>,
    pub auto_tag: bool,

    // Deposit options
    pub organize: DepositMode,
    pub target_dir: Option<PathBuf>,

    // Process options
    pub steps: Option<Vec<String>>,
}

impl Config {
    fn parse_library_and_command(
        &mut self,
        args: &mut impl Iterator<Item = String>,
    ) -> types::UnitResult {
        let arg = args.next();
        if arg.is_none() {
            return Ok(()); // 'help' is default
        }

        let arg = arg.unwrap();
        match arg.as_str() {
            "help" | "h" | "-h" | "--help" => return Ok(()), // 'help' is default
            "list" | "ls" | "l" => self.command = String::from("list"),
            _ => {
                self.command = args.next().unwrap_or(String::from("show"));
                self.setup_library(arg)?;
            }
        }

        Ok(())
    }

    /// Set up the library and its configuration paths.
    ///
    /// # Errors
    /// When the library folder is not found. This applies to every library command, except `add`, which will
    /// create it.
    fn setup_library(&mut self, library: String) -> types::UnitResult {
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

            let option = line.split_once("=");
            if option.is_none() {
                return Err(format!("Invalid config line: {}", line).into());
            }

            let (key, value) = option.unwrap();
            match key.to_lowercase().as_str() {
                // General
                "description" => self.lib_desc = Some(String::from(value)),
                "verbose" => self.verbose = value.parse::<bool>()?,
                // Download
                "clear_input" => self.clear_input = value.parse::<bool>()?,
                "auto_download" => self.auto_download = value.parse::<bool>()?,
                // Tag
                "override_artist" => self.override_artist = value.parse::<bool>()?,
                "filename_template" => self.filename_template = String::from(value),
                "title_template" => self.title_template = String::from(value),
                "auto_tag" => self.auto_tag = value.parse::<bool>()?,
                // Tag, Deposit
                "input_dir" => self.input_dir = Some(PathBuf::from(value)),
                // Deposit
                "target_dir" => self.target_dir = Some(PathBuf::from(value)),
                "organize" => self.organize = DepositMode::from(value)?,
                // Process
                "steps" => self.steps = Some(value.split(',').map(String::from).collect()),
                _ => return Err(format!("Invalid config option: {}", key).into()),
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
                        self.clear_input = true;
                    }
                    't' if self.command == "tag" => self.auto_tag = true,
                    'i' if self.command == "tag"
                        || self.command == "deposit"
                        || self.command == "process" =>
                    {
                        self.input_dir = args.next().map(PathBuf::from);
                    }
                    'd' if self.command == "deposit" || self.command == "process" => {
                        if let Some(mode) = args.next() {
                            self.organize = DepositMode::from(mode.as_str())?;
                        } else {
                            return Err("Organization mode not specified. See 'help'".into());
                        }
                    }
                    'o' if self.command == "deposit" || self.command == "process" => {
                        self.target_dir = args.next().map(PathBuf::from);
                    }
                    's' if self.command == "process" => {
                        self.steps =
                            Some(args.next().unwrap().split(',').map(String::from).collect());
                    }
                    _ => {
                        return Err(format!(
                            "Unrecognized option '{}' for command '{}'. See 'help'",
                            c, self.command
                        )
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    fn require_input_dir(&mut self) -> types::UnitResult {
        if self.input_dir.is_none() {
            return Err("Input directory not specified. See 'help'".into());
        }

        let lib_path = self.lib_path.as_ref();
        self.input_dir = Some(lib_path.unwrap().join(self.input_dir.as_ref().unwrap()));
        let input_dir = self.input_dir.as_ref().unwrap();
        if fs::metadata(input_dir).is_err() {
            return Err(format!("Input directory not found: {}", input_dir.display()).into());
        }

        Ok(())
    }

    fn require_target_dir(&mut self) -> types::UnitResult {
        if self.target_dir.is_none() {
            return Err("Target directory not specified. See 'help'".into());
        }

        let lib_path = self.lib_path.as_ref();
        self.target_dir = Some(lib_path.unwrap().join(self.target_dir.as_ref().unwrap()));
        let target_dir = self.target_dir.as_ref().unwrap();
        if fs::metadata(target_dir).is_err() {
            return Err(format!("Target directory not found: {}", target_dir.display()).into());
        }

        Ok(())
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> types::ConfigResult {
        args.next(); // Consume program name

        let mut config = Config {
            command: String::from("help"),
            title_template: String::from("{title} ({feat}) [{remix}]"),
            filename_template: String::from("{artist} - {title}"),
            ..Default::default()
        };
        config.parse_library_and_command(&mut args)?;

        // Parse extra options for commands that have them
        if config.command == "add" {
            config.parse_terms(args)?;
        } else if config.command == "show" {
            config.build_lib_conf_options()?; // just load the library settings
        } else if ["download", "tag", "deposit", "process"].contains(&config.command.as_str()) {
            config.build_lib_conf_options()?; // override defaults with lib.conf
            config.parse_cli_options(args)?; // override defaults/lib.conf with CLI
            config.require_input_dir()?;
        }

        if config.command.as_str() == "deposit" {
            config.require_target_dir()?;
        }

        Ok(config)
    }

    fn steps(&self) -> Result<Vec<&String>, Box<dyn std::error::Error>> {
        if self.command.as_str() != "process" {
            return Ok(vec![&self.command]);
        }
        if self.steps.is_none() {
            return Err("No processing steps specified. See 'help'".into());
        }

        let steps = self.steps.as_ref().unwrap();
        let mut commands = Vec::with_capacity(steps.len());
        for step in steps {
            if ["download", "tag", "deposit"].contains(&step.as_str()) {
                commands.push(step);
            } else {
                return Err(format!("Unsupported processing step '{}'. See 'help'", step).into());
            }
        }
        Ok(commands)
    }
}

pub fn run<R: BufRead>(config: Config, mut reader: R) -> types::UnitResult {
    for command in &config.steps()? {
        match command.as_str() {
            "help" => info::help(),
            "list" => info::list()?,
            "show" => info::show(&config)?,
            "add" => add::run(&config)?,
            "download" => download::run(&config, &mut reader)?,
            "tag" => tag::run(&config, &mut reader)?,
            "deposit" => deposit::run(&config, &mut reader)?,
            _ => return Err("Unrecognized command. See 'help'".into()),
        }
    }
    Ok(())
}
