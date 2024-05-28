//! Move (downloaded and/or tagged) files to a target directory.

use crate::util::PromptOption::{No, Yes};
use crate::{types, util, Config};
use audiotags::Tag;
use chrono::{DateTime, Datelike, Utc};
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;

type BuildTargetFunction = fn(&PathBuf, &PathBuf) -> types::PathBufResult;

#[derive(Debug, PartialEq)]
pub enum DepositMode {
    /// Sort files into `A-Z/ARTIST?/ALBUM?` subfolders
    AZ,
    /// Sort files into `YYYY/MM` subfolders
    Date,
    /// Drop files directly in `target_dir`
    Drop,
}

impl Default for DepositMode {
    fn default() -> Self {
        Self::Drop
    }
}

impl DepositMode {
    pub fn from(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match s {
            "A-Z" => Ok(Self::AZ),
            "DATE" => Ok(Self::Date),
            "DROP" => Ok(Self::Drop),
            _ => Err(format!("Invalid organization mode: '{}'. See 'help'", s).into()),
        }
    }

    fn func(&self) -> BuildTargetFunction {
        match self {
            Self::AZ => alphabetical,
            Self::Date => chronological,
            Self::Drop => drop,
        }
    }
}

/// Attempt to move all (downloaded and processed) files (not directories) in `INPUT_DIR` to
/// `TARGET_DIR`. If the target folder does not exist, it is created. If a file already exists in
/// the target folder, it will be overwritten upon user confirmation.
pub fn run<R: BufRead>(config: &Config, reader: R) -> types::UnitResult {
    if config.target_dir.is_none() {
        return Err("'TARGET_DIR' required for moving downloads. See 'help'".into());
    } else if config.input_dir.is_none() {
        return Err("'INPUT_DIR' required for moving downloads to 'TARGET_DIR'. See 'help'".into());
    }

    let lib_path = config.lib_path.clone().unwrap();

    let downloads = util::filepaths_in(lib_path.join(config.input_dir.clone().unwrap()))?;
    if downloads.is_empty() {
        return Ok(());
    }

    let target_dir = util::guarantee_dir_path(lib_path.join(config.target_dir.clone().unwrap()))?;

    if let Some(errors) = deposit(target_dir, downloads, config.organize.func(), reader) {
        return Err(format!(
            "Could not move {} files to target directory:{}",
            errors.len(),
            errors.iter().fold(String::new(), |a, b| a + "\n" + &b)
        )
        .into());
    }

    println!();

    Ok(())
}

/// Sort the `file` into a dated subfolder of `target_dir`:
/// `target_dir/YYYY/MM/file.ext`, where `YYYY` and `MM` are determined from file creation date.
///
/// Examples:
/// - `randomfile.jpg` created at 2024-04-29    -> `target_dir/2024/04/randomfile.jpg`
/// - `Artist - Song.mp3` created at 2024-05-15 -> `target_dir/2024/05/Artist - Song.mp3`
fn chronological(target_dir: &PathBuf, file: &PathBuf) -> types::PathBufResult {
    let filename = file.file_name().unwrap().to_owned().into_string().unwrap();

    let target = if let Ok(meta) = fs::metadata(&file) {
        if let Ok(created) = meta.created() {
            let created: DateTime<Utc> = created.into();
            target_dir
                .join(created.year().to_string())
                .join(format!("{:02}", created.month()))
        } else {
            return Err("! Unsupported platform: can't get file date".into());
        }
    } else {
        return Err(format!("! Invalid path or no permission: {}", filename).into());
    };

    Ok(util::guarantee_dir_path(target)?.join(filename))
}

/// Sort the `file` into an alphabetical subfolder of `target_dir`:
/// `target_dir/A-Z/ARTIST?/ALBUM?/file.ext`, where ARTIST and ALBUM are optional (determined from
/// file tags). The letter `A-Z` subfolder is based on the ARTIST tag. If the ARTIST tag is not
/// present, the artist is guessed from the filename (if there is a part to the left of a '-'
/// separator). If that fails, the first letter of the filename is used.
///
/// Examples:
/// - `randomfile.jpg`                         -> `target_dir/R/randomfile.jpg`
/// - `Song.mp3 with artist tag 'Band'`        -> `target_dir/B/Band/Song.mp3`
/// - `Song.mp3 without artist tag`            -> `target_dir/S/Song.mp3`
/// - `Band - Song.mp3 with artist tag 'Band'` -> `target_dir/B/Band/Band - Song.mp3`
/// - `Band - Song.mp3 without artist tag`     -> `target_dir/B/Band/Band - Song.mp3`
/// - `Band - Song.mp3 with artist, album tag` -> `target_dir/B/Band/Album/Band - Song.mp3`
fn alphabetical(target_dir: &PathBuf, file: &PathBuf) -> types::PathBufResult {
    let filename = file.file_name().unwrap().to_owned().into_string().unwrap();
    let tag = Tag::new().read_from_path(&file);

    let mut target = None;
    if let Ok(tag) = &tag {
        // Attempt to get the ARTIST from tag
        if let Some(artist) = tag.artist() {
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
    if target.is_some() {
        // Now that ARTIST is set, try to also set the ALBUM subfolder (from tag)
        if let Ok(tag) = &tag {
            if let Some(album) = tag.album_title() {
                target = Some(target.unwrap().join(album));
            }
        }
    } else {
        // No ARTIST, default to 'LETTER/' subfolder only
        target = Some(target_dir.join(letter_for(&filename)));
    }

    Ok(util::guarantee_dir_path(target.unwrap())?.join(filename))
}

/// Drop the `file` file directly in `target_dir`.
fn drop(target_dir: &PathBuf, file: &PathBuf) -> types::PathBufResult {
    Ok(target_dir.join(file.file_name().unwrap().to_owned().into_string().unwrap()))
}

fn deposit<R: BufRead>(
    target_dir: PathBuf,
    downloads: Vec<PathBuf>,
    func: BuildTargetFunction,
    mut reader: R,
) -> types::OptionVecString {
    println!("Moving files to {}...", target_dir.display());

    let mut errors = Vec::new();

    for entry in downloads {
        println!();

        let target = func(&target_dir, &entry);
        if let Err(e) = target {
            errors.push(format!(
                "! Could not create target dir: {}\n    {}",
                target_dir.display(),
                e
            ));
            continue;
        }
        let target = target.unwrap();

        if !overwrite(&target, &mut reader) {
            println!("  Skipping {}", entry.display());
            continue;
        }

        if fs::rename(&entry, &target).is_ok() {
            println!("  {}\n> {}", entry.display(), target.display());
        } else {
            errors.push(format!("! {}\n> {}", entry.display(), target.display()));
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

fn letter_for(s: &str) -> String {
    let letter = s.chars().nth(0).unwrap().to_ascii_uppercase();
    if "ABCDEFGHIJKLMNOPQRSTUVWXYZ".contains(letter) {
        String::from(letter)
    } else {
        String::from("0-9#") // symbols and 'weird letters'
    }
}

/// Checks if a file already exists at the `target` location,
/// and asks the user whether to overwrite it.
///
/// # Returns
/// - `true` when the file does not exist, or to overwrite it if it does
/// - `false` when the file exists and the user does not want to overwrite it
fn overwrite<R: BufRead>(target: &PathBuf, reader: R) -> bool {
    if fs::metadata(target).is_err() {
        return true;
    }
    let prompt = format!(
        "! File already exists: {}\nOverwrite?",
        target.to_str().unwrap()
    );
    match util::select(&prompt, vec![Yes, No], Yes, reader) {
        Ok(Yes) => true,
        _ => false, // Don't overwrite on Err(_) or Ok(No)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercases_letter() {
        for letter in "abcdefghijklmnopqrstuvwxyz".chars() {
            assert_eq!(
                letter_for(&letter.to_string()),
                letter.to_ascii_uppercase().to_string()
            );
        }
    }

    #[test]
    fn handles_non_letters() {
        for symbol in ["42", "2U", ".band.", "アーティスト", "歌手"] {
            assert_eq!(letter_for(symbol), String::from("0-9#"));
        }
    }
}
