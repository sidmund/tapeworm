mod add;
mod alias;
mod clean;
mod command;
mod deposit;
mod download;
mod editor;
mod info;
mod scrape;
mod tag;
mod types;
mod util;

use crate::command::Command::{self, *};
use crate::deposit::DepositMode;
use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Default)]
pub struct Config {
    pub commands: Vec<Command>,
    pub lib_alias: Option<String>,
    pub lib_desc: Option<String>,
    pub aliases: BTreeMap<String, PathBuf>,

    // Paths
    pub general_conf: PathBuf,
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
    pub auto_overwrite: bool,
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

        if let Ok(cmd) = Command::from(arg.as_ref().unwrap()) {
            if cmd == List {
                self.commands = vec![cmd];
                self.parse_general_config()?;
            } else if cmd != Help {
                // Invoked as `tapeworm COMMAND [OPTIONS]`
                self.commands = vec![cmd];
                self.setup_library(None)?;
            }
        } else {
            // Invoked as `tapeworm LIBRARY [COMMAND] [OPTIONS]`
            self.setup_library(Some(arg.unwrap()))?;
            self.commands = if let Some(arg) = args.next() {
                vec![Command::from(&arg).unwrap()]
            } else {
                vec![Show] // The default when only LIBRARY given
            };
        }

        Ok(()) // 'help' ends up here immediately as it is the default
    }

    /// Parse extra options for commands that require them.
    fn parse_extra_options(&mut self, args: impl Iterator<Item = String>) -> types::UnitResult {
        // Load library settings (overrides defaults)
        if self.commands[0].uses_lib_conf() {
            self.build_lib_conf_options()?;
        }

        // Parse CLI options (may override defaults/lib.conf)
        if self.commands[0].uses_cli() {
            self.parse_cli_options(args)?;
        } else if self.commands[0] == Add {
            let terms = args.collect::<Vec<String>>();
            if terms.is_empty() {
                return Err("Provide search term(s) and/or URL(s). See 'help'".into());
            }
            self.terms = Some(terms);
        } else if self.commands[0] == Alias {
            let terms = args.collect::<Vec<String>>();
            if !terms.is_empty() {
                self.terms = Some(terms);
            }
        }

        // Enforce parameter requirements
        if self.commands[0] == Process {
            // When lib.conf and CLI did not receive 'steps'
            return Err("Steps not specified. See 'help'".into());
        }
        if self.commands.contains(&Tag) || self.commands.contains(&Deposit) {
            self.require_input_dir()?;
        }
        if self.commands.contains(&Deposit) || self.commands.contains(&Clean) {
            self.require_target_dir()?;
        }
        Ok(())
    }

    /// Read in the configured aliases.
    fn parse_general_config(&mut self) -> types::UnitResult {
        if let Some(contents) = fs::read_to_string(&self.general_conf).ok() {
            for line in contents.lines().map(|l| l.trim()) {
                if line.is_empty() || line.starts_with("#") {
                    continue;
                }

                if let Some((aka, path)) = line.split_once("=") {
                    self.aliases.insert(String::from(aka), PathBuf::from(path));
                } else {
                    return Err(format!("Invalid alias: {}", line).into());
                }
            }
        }
        Ok(())
    }

    /// Set up the library and its configuration paths for commands that require it.
    fn setup_library(&mut self, library: Option<String>) -> types::UnitResult {
        self.parse_general_config()?;

        let lib_path = if let Some(library) = library {
            // Assume 'library' to be an alias pointing to the library path,
            // else assume 'library' to be the library path itself
            if let Some(lib_path) = self.aliases.get(&library) {
                self.lib_alias = Some(library);
                if lib_path.starts_with("~/") {
                    let rest = &lib_path.to_str().unwrap()[2..];
                    dirs::home_dir().unwrap().join(rest)
                } else {
                    lib_path.clone()
                }
            } else {
                env::current_dir()?.join(library)
            }
        } else {
            env::current_dir()? // Assume current directory to be a library
        };

        let lib_conf_folder = lib_path.join(".tapeworm");
        if fs::metadata(&lib_conf_folder).is_err() {
            return Err(format!("Not a library folder: {}", lib_path.to_str().unwrap()).into());
        }

        self.lib_conf_path = Some(lib_conf_folder.join("lib.conf"));
        self.input_path = Some(lib_conf_folder.join("input.txt"));
        self.yt_dlp_conf_path = Some(lib_conf_folder.join("yt-dlp.conf"));
        self.input_dir = Some(lib_conf_folder.join("tmp"));
        self.target_dir = Some(lib_path.clone());
        self.lib_path = Some(lib_path);

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
                "auto_overwrite" => self.auto_overwrite = value.parse::<bool>()?,
                // Process
                "steps" => self.parse_steps(Some(String::from(value)))?,
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
                    'c' if [Download, Process].contains(&self.commands[0]) => {
                        self.clear_input = true;
                    }
                    'a' if [Download, Process].contains(&self.commands[0]) => {
                        self.auto_download = true;
                    }
                    't' if [Tag, Process].contains(&self.commands[0]) => self.auto_tag = true,
                    'i' if [Tag, Deposit, Process].contains(&self.commands[0]) => {
                        self.input_dir = args.next().map(PathBuf::from);
                    }
                    'd' if [Deposit, Process].contains(&self.commands[0]) => {
                        if let Some(mode) = args.next() {
                            self.organize = DepositMode::from(mode.as_str())?;
                        } else {
                            return Err("Organization mode not specified. See 'help'".into());
                        }
                    }
                    'o' if [Deposit, Clean, Process].contains(&self.commands[0]) => {
                        self.target_dir = args.next().map(PathBuf::from);
                    }
                    's' if self.commands[0] == Process => self.parse_steps(args.next())?,
                    _ => {
                        return Err(format!(
                            "Unrecognized option '{}' for command '{:?}'. See 'help'",
                            c, self.commands[0]
                        )
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_steps(&mut self, steps: Option<String>) -> types::UnitResult {
        if self.commands[0] != Process {
            return Ok(());
        }
        if steps.is_none() {
            return Err("Steps not specified. See 'help'".into());
        }

        let mut commands = Vec::new();
        for step in steps.unwrap().split(',') {
            let cmd = Command::from(step)?;
            if !cmd.is_valid_processing_step() {
                return Err(format!("Unsupported process step '{:?}'. See 'help'", cmd).into());
            }
            commands.push(cmd);
        }

        if commands.is_empty() {
            Err("Steps not specified. See 'help'".into())
        } else {
            self.commands = commands;
            Ok(())
        }
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

    fn default() -> Self {
        Self {
            commands: vec![Help],
            general_conf: PathBuf::from(dirs::config_dir().unwrap())
                .join("tapeworm")
                .join("tapeworm.conf"),
            title_template: String::from("{title} ({feat}) [{remix}]"),
            filename_template: String::from("{artist} - {title}"),
            ..Default::default()
        }
    }

    pub fn build(mut args: impl Iterator<Item = String>) -> types::ConfigResult {
        args.next(); // Consume program name

        let mut config = Config::default();
        config.parse_library_and_command(&mut args)?;
        config.parse_extra_options(args)?;
        Ok(config)
    }
}

pub fn run<R: BufRead>(config: Config, mut reader: R) -> types::UnitResult {
    for cmd in &config.commands {
        match cmd {
            Help => info::help(),
            List => info::list(&config),
            Alias => alias::run(&config)?,
            Show => info::show(&config)?,
            Clean => clean::run(&config)?,
            Add => add::run(&config)?,
            Download => download::run(&config, &mut reader)?,
            Tag => tag::run(&config, &mut reader)?,
            Deposit => deposit::run(&config, &mut reader)?,
            _ => return Err(format!("Cannot run this command: {:?}. See 'help'", cmd).into()),
        }
    }
    Ok(())
}
