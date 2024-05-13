//! Move (downloaded and/or tagged) files to a target directory.

use crate::types;
use crate::util;
use crate::Config;
use audiotags::Tag;
use std::fs;
use std::path::PathBuf;

/// Attempt to move all downloaded (and processed) files in YT_DLP_OUTPUT_DIR to TARGET_DIR.
/// TARGET_DIR is created if not present.
/// Directories are not moved, only files.
/// If a file already exists in TARGET_DIR, it will be overwritten.
///
/// If DEPOSIT_AZ is enabled, files will be moved to organized subdirectories of TARGET_DIR.
pub fn deposit(config: &Config) -> types::UnitResult {
    if config.target_dir.is_none() {
        return Err("'TARGET_DIR' must be set for moving downloads. See 'help'".into());
    } else if config.yt_dlp_output_dir.is_none() {
        return Err(
            "'YT_DLP_OUTPUT_DIR' must be set for moving downloads to 'TARGET_DIR'. See 'help'"
                .into(),
        );
    }

    let downloads = PathBuf::from(config.lib_path.clone().unwrap())
        .join(config.yt_dlp_output_dir.clone().unwrap());
    let downloads: Vec<PathBuf> = util::filepaths_in(downloads).unwrap_or(vec![]);
    if downloads.is_empty() {
        return Ok(());
    }

    let target_dir =
        PathBuf::from(config.lib_path.clone().unwrap()).join(config.target_dir.clone().unwrap());
    let target_dir = util::guarantee_dir_path(target_dir)?;

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

/// Organize the `downloads` files into subfolders of `target_dir`.
/// If the 'artist' tag is present, a subfolder for the artist is created.
/// If the tag is not present, the artist is guessed from the filename,
/// i.e. the part to the left of '-'.
/// If that fails, the first letter of the filename is used.
///
/// Examples:
/// - `randomfile.jpg`                         -> `target_dir/R/randomfile.jpg`
/// - `Song.mp3 with artist tag 'Band'`        -> `target_dir/B/Band/Song.mp3`
/// - `Song.mp3 without artist tag`            -> `target_dir/S/Song.mp3`
/// - `Band - Song.mp3 with artist tag 'Band'` -> `target_dir/B/Band/Band - Song.mp3`
/// - `Band - Song.mp3 without artist tag`     -> `target_dir/B/Band/Band - Song.mp3`
fn organize(target_dir: PathBuf, downloads: Vec<PathBuf>) -> types::OptionVecString {
    println!("Sorting files into {}...", target_dir.display());

    let mut errors = Vec::new();

    for entry in downloads {
        let filename = entry.file_name().unwrap().to_owned().into_string().unwrap();

        let mut target = None;
        // Attempt to get the 'LETTER/ARTIST/' subfolders
        if let Ok(tag) = Tag::new().read_from_path(&entry) {
            if let Some(artist) = tag.artist() {
                // '.' cannot appear last in folder name
                let artist = if artist.ends_with('.') {
                    &artist[..artist.len() - 1]
                } else {
                    artist
                };
                target = Some(target_dir.join(letter_for(artist)).join(artist));
            }
        }
        if target.is_none() {
            // Attempt to get the ARTIST from filename
            if let Some((author, _)) = filename.split_once('-') {
                let author = author.trim();
                if !author.is_empty() {
                    target = Some(target_dir.join(letter_for(&author)).join(author));
                }
            }
        }
        if target.is_none() {
            // Default to 'LETTER/' subfolder only
            target = Some(target_dir.join(letter_for(&filename)));
        }

        let target_path = target.clone().unwrap();
        let target = util::guarantee_dir_path(target.unwrap());
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

        if let Some(error) = rename(entry, target) {
            errors.push(error);
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

        if let Some(error) = rename(entry, target) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

/// Attempt to rename (move) the `entry` file to `target` file.
///
/// # Returns
/// - `None` when successful
/// - `Some(String)` with a file error message
fn rename(entry: PathBuf, target: PathBuf) -> Option<String> {
    if fs::rename(entry.clone(), target.clone()).is_err() {
        Some(format!(
            "! {}\n    -> {}",
            entry.display(),
            target.display()
        ))
    } else {
        println!("  {} -> {}", entry.display(), target.display());
        None
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
