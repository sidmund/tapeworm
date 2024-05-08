//! Download all inputs in the library.
//! After downloading, the following steps may optionally be performed:
//! - Tag the downloaded files
//! - Move the (tagged) downloaded files to a target directory

use crate::tag;
use crate::types;
use crate::Config;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn download(config: &Config) -> types::UnitResult {
    download_inputs(&config)?;
    tag::tag(&config)?;
    deposit(&config)
}

fn download_inputs(config: &Config) -> types::UnitResult {
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

    let inputs: HashSet<String> = inputs.lines().map(|s| s.to_string()).collect();

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

/// Attempt to move all downloaded (and processed) files in YT_DLP_OUTPUT_DIR to TARGET_DIR.
/// TARGET_DIR is created if not present.
/// Directories are not moved, only files.
/// If a file already exists in TARGET_DIR, it will be overwritten.
fn deposit(config: &Config) -> types::UnitResult {
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
