//! Download all inputs in the library.

use crate::types;
use crate::util;
use crate::Config;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::process::{Command, Stdio};

pub fn download(config: &Config) -> types::UnitResult {
    let use_yt_dlp_conf = if let Some(value) = yt_dlp_conf_exists(config)? {
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

/// Returns:
/// - Some(true) if yt-dlp.conf exists, it will be used
/// - Some(false) if the user wants to continue without yt-dlp.conf
/// - None if the user wants to abort
fn yt_dlp_conf_exists(config: &Config) -> types::OptionBoolResult {
    if fs::metadata(&config.yt_dlp_conf_path.clone().unwrap()).is_ok() {
        return Ok(Some(true));
    }

    println!(
            "Warning: {} not found
If you continue, yt-dlp will be invoked without any options, which will yield inconsistent results.",
            config.yt_dlp_conf_path.clone().unwrap().to_str().unwrap()
        );

    if util::confirm("Do you want to continue regardless?", false)? {
        Ok(Some(false))
    } else {
        Ok(None)
    }
}
