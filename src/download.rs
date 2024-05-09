//! Download all inputs in the library.
//! After downloading, the following steps may optionally be performed:
//! - Tag the downloaded files
//! - Move the (tagged) downloaded files to a target directory

use crate::tag;
use crate::types;
use crate::util;
use crate::Config;
use audiotags::Tag;
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
///
/// If DEPOSIT_AZ is enabled, files will be moved to organized subdirectories of TARGET_DIR.
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
    let target_dir = util::guarantee_dir_path(target_dir)?;

    let downloads = PathBuf::from(config.lib_path.clone().unwrap())
        .join(config.yt_dlp_output_dir.clone().unwrap());
    let downloads: Vec<PathBuf> = fs::read_dir(downloads)?
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|t| t.file_type().is_ok_and(|f| f.is_file()))
        })
        .map(|e| e.unwrap().path())
        .collect();

    if downloads.is_empty() {
        return Ok(());
    }

    if let Some(errors) = if config.deposit_az {
        organize(target_dir, downloads)
    } else {
        drop(target_dir, downloads)
    } {
        return Err(format!(
            "Could not move {} files to target directory:{}",
            errors.len(),
            errors.iter().fold(String::new(), |a, b| a + "\n" + &b)
        )
        .into());
    }

    Ok(())
}

/// Organize the `downloads` files into subfolders of `target_dir`. This is based
/// on the artist tag of a file, or (the first letter of) the filename if the tag
/// is not present.
///
/// Example files:
/// - `randomfile.jpg`                         -> `target_dir/R/randomfile.jpg`
/// - `Song.mp3 with artist tag 'Band'`        -> `target_dir/B/Band/Song.mp3`
/// - `Band - Song.mp3 with artist tag 'Band'` -> `target_dir/B/Band/Band - Song.mp3`
/// - `Band - Song.mp3 without artist tag`     -> `target_dir/B/Band - Song.mp3`
fn organize(target_dir: PathBuf, downloads: Vec<PathBuf>) -> types::OptionVecString {
    println!("Sorting files into {}...", target_dir.display());

    let mut errors = Vec::new();

    for entry in downloads {
        let filename = entry.file_name().unwrap().to_owned().into_string().unwrap();

        let target = if let Ok(tag) = Tag::new().read_from_path(&entry) {
            if let Some(artist) = tag.artist() {
                // '.' cannot appear last in folder name
                let artist = if artist.ends_with('.') {
                    &artist[..artist.len() - 1]
                } else {
                    artist
                };
                let letter = letter_for(artist);

                target_dir.join(letter).join(artist)
            } else {
                target_dir.join(letter_for(&filename))
            }
        } else {
            target_dir.join(letter_for(&filename))
        };

        let target_path = target.clone();
        let target = util::guarantee_dir_path(target);
        if target.is_err() {
            errors.push(format!(
                "! Could not create target dir: {}\n    {}",
                target_path.display(),
                target.unwrap_err()
            ));
            continue;
        }
        let target = target.unwrap().join(filename);

        if !overwrite(&target) {
            println!("  Skipping {}", entry.display());
            continue;
        }

        if fs::rename(entry.clone(), target.clone()).is_err() {
            errors.push(format!(
                "! {}\n    -> {}",
                entry.display(),
                target.display()
            ));
        } else {
            println!("  {} -> {}", entry.display(), target.display());
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

/// Simply drop the `downloads` files directly in `target_dir`.
fn drop(target_dir: PathBuf, downloads: Vec<PathBuf>) -> types::OptionVecString {
    println!("Dropping files into {}...", target_dir.display());

    let mut errors = Vec::new();

    for entry in downloads {
        let filename = entry.file_name().unwrap().to_owned().into_string().unwrap();

        let target = target_dir.join(filename);

        if !overwrite(&target) {
            println!("  Skipping {}", entry.display());
            continue;
        }

        if fs::rename(entry.clone(), target.clone()).is_err() {
            errors.push(format!(
                "! {}\n    -> {}",
                entry.display(),
                target.display()
            ));
        } else {
            println!("  {} -> {}", entry.display(), target.display());
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

fn letter_for(s: &str) -> String {
    let mut letter = String::from(&s[..1].to_uppercase());
    if !"ABCDEFGHIJKLMNOPQRSTUVWXYZ".contains(letter.as_str()) {
        letter = String::from("0-9#"); // symbols and 'weird letters'
    }
    letter
}

/// Checks if a file already exists at the `target` location,
/// and asks the user whether to overwrite it.
/// Returns true to overwrite, false otherwise.
fn overwrite(target: &PathBuf) -> bool {
    if fs::metadata(target).is_ok() {
        let overwrite = util::confirm("The file already exists. Overwrite?", true);
        if overwrite.is_err() || !overwrite.unwrap() {
            return false;
        }
    }
    true
}
