//! Download all inputs in the library.

use crate::util::PromptOption::{No, Yes, YesToAll};
use crate::{types, util, Config};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn download<R: BufRead>(config: &Config, mut reader: R) -> types::UnitResult {
    let mut yt_dlp_conf_path = config.yt_dlp_conf_path.clone();
    if fs::metadata(yt_dlp_conf_path.as_ref().unwrap()).is_err() {
        println!(
            "Warning: {} not found
If you continue, yt-dlp will be invoked without any options, which will yield inconsistent results.",
            yt_dlp_conf_path.unwrap().to_str().unwrap()
        );
        if !util::confirm("Do you want to continue regardless?", false, &mut reader)? {
            return Ok(()); // User wants to abort when config is not found
        }
        yt_dlp_conf_path = None;
    }

    let input_path = config.input_path.clone().unwrap();
    if fs::metadata(&input_path).is_err() {
        return Err(format!("Input file not found: {}", input_path.to_str().unwrap()).into());
    }

    let inputs = fs::read_to_string(&input_path)?;
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
    yt_dlp(yt_dlp_conf_path, inputs)?;

    if config.clear_input {
        fs::write(&input_path, "")?;
    }

    if config.confirm_downloads {
        confirm_downloads(config, &mut reader)?;
    }

    Ok(())
}

/// Download URLs with yt-dlp
fn yt_dlp(conf_path: Option<PathBuf>, urls: HashSet<String>) -> types::UnitResult {
    let mut command = Command::new("yt-dlp");
    if let Some(conf_path) = conf_path {
        command.arg("--config-location").arg(conf_path);
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

fn confirm_downloads<R: BufRead>(config: &Config, mut reader: R) -> types::UnitResult {
    let lib_path = config.lib_path.clone().unwrap();

    let downloads = lib_path.join(config.input_dir.clone().unwrap());
    let downloads: Vec<PathBuf> = util::filepaths_in(downloads)?;
    if downloads.is_empty() {
        return Ok(());
    }
    let total = downloads.len();

    println!("Downloaded {} files:", total);
    downloads
        .iter()
        .for_each(|d| println!("{}", d.to_str().unwrap()));

    for (i, entry) in downloads.iter().enumerate() {
        println!("\nFile {} of {}: {}", i, total, entry.to_str().unwrap());
        let choice = util::confirm_with_options(
            "Keep this?",
            vec![Yes, No, YesToAll],
            YesToAll,
            &mut reader,
        )?;
        match choice {
            No => {
                fs::remove_file(entry)?;
                println!("Deleted {}.", entry.to_str().unwrap());
            }
            Yes => continue,
            YesToAll => break,
        }
    }

    Ok(())
}
