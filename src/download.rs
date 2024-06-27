use crate::util::PromptOption::{No, Yes, YesToAll};
use crate::{types, util, Config};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Interface for downloading files.
pub trait Downloader {
    fn download<R: BufRead>(
        &self,
        config: &Config,
        inputs: HashSet<String>,
        reader: R,
    ) -> types::UnitResult;
}

/// Wrapper for `yt-dlp`.
pub struct YtDlp;

impl YtDlp {
    fn get_config<R: BufRead>(config: &Config, mut reader: R) -> Option<&PathBuf> {
        let mut yt_dlp_conf_path = config.yt_dlp_conf_path.as_ref();
        if fs::metadata(yt_dlp_conf_path.unwrap()).is_err() {
            println!(
                "Warning! Could not find: {}\nIf you continue, yt-dlp will be invoked without any options, which will yield inconsistent results.",
                yt_dlp_conf_path.unwrap().to_str().unwrap()
            );
            match util::select("Continue anyway?", vec![Yes, No], No, &mut reader) {
                Ok(Yes) => yt_dlp_conf_path = None,
                _ => std::process::exit(0), // User wants to abort when config is not found
            }
        }
        yt_dlp_conf_path
    }
}

impl Downloader for YtDlp {
    fn download<R: BufRead>(
        &self,
        config: &Config,
        inputs: HashSet<String>,
        mut reader: R,
    ) -> types::UnitResult {
        let mut command = Command::new("yt-dlp");
        if let Some(conf_path) = YtDlp::get_config(config, &mut reader) {
            command.arg("--config-location").arg(conf_path);
        }
        inputs.iter().for_each(|url| {
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
}

pub fn run<R, D>(config: &Config, mut reader: R, downloader: &D) -> types::UnitResult
where
    R: BufRead,
    D: Downloader,
{
    if let Some(inputs) = get_inputs(config) {
        downloader.download(config, inputs, &mut reader)?;
    } else {
        if config.verbose {
            println!("Nothing to download. Library is empty.");
        }
        return Ok(());
    }

    if config.clear_input {
        fs::write(config.input_path.as_ref().unwrap(), "")?;
    }

    if config.auto_download {
        Ok(())
    } else {
        confirm_downloads(config, &mut reader)
    }
}

fn get_inputs(config: &Config) -> Option<HashSet<String>> {
    let input_path = config.input_path.as_ref().unwrap();
    let inputs = fs::read_to_string(input_path).unwrap_or(String::new());
    if inputs.is_empty() {
        return None;
    }

    let inputs: HashSet<String> = inputs.lines().map(|s| s.to_string()).collect();
    if config.verbose {
        println!("Downloading {} URLs:", inputs.len());
        inputs.iter().for_each(|s| println!("  {}", s));
        println!();
    }
    Some(inputs)
}

fn confirm_downloads<R: BufRead>(config: &Config, mut reader: R) -> types::UnitResult {
    let downloads: Vec<PathBuf> = util::filepaths_in(config.input_dir.as_ref().unwrap())?;
    if downloads.is_empty() {
        return Ok(());
    }
    let total = downloads.len();

    println!("\nDownloaded {} files:", total);
    downloads
        .iter()
        .for_each(|d| println!("  {}", d.to_str().unwrap()));

    for (i, entry) in downloads.iter().enumerate() {
        println!("\nFile {} of {}: {}", i + 1, total, entry.to_str().unwrap());
        let choice = util::select("Keep?", vec![Yes, No, YesToAll], YesToAll, &mut reader);
        match choice {
            Ok(No) => {
                fs::remove_file(entry)?;
                println!("Deleted {}", entry.to_str().unwrap());
            }
            Ok(Yes) => continue,
            _ => break, // Keep all on Err(_) or Ok(YesToAll)
        }
    }

    Ok(())
}
