//! Download all inputs in the library.

use crate::organize;
use crate::tag;
use crate::types;
use crate::Config;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::process::{Command, Stdio};

pub fn download(config: &Config) -> types::UnitResult {
    download_inputs(&config)?;
    tag::tag(&config)?;
    organize::deposit(&config)
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
