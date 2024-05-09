//! Utility functions.

use crate::types;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Read a line from stdin.
/// The line is trimmed and converted to lowercase.
pub fn input() -> types::StringResult {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase())
}

/// Prompt the user for confirmation.
///
/// # Returns
/// - `default` when the user presses 'Enter'
/// - `true` if the user enters "y"
/// - `false` if the user enters anything else
pub fn confirm(prompt: &str, default: bool) -> types::BoolResult {
    println!("{} {}", prompt, if default { "Y/n" } else { "y/N" });
    let input = input()?;
    if input.is_empty() {
        Ok(default)
    } else {
        Ok(input.starts_with('y'))
    }
}

/// Append the `contents` to the file at `path`.
pub fn append<P>(path: P, contents: String) -> types::UnitResult
where
    P: AsRef<Path>,
{
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(contents.as_bytes())?;
    Ok(())
}

/// If the path `dir` does not exist, it is created.
/// Returns ownership of `dir`, guaranteed to be an existing directory.
pub fn guarantee_dir_path(dir: PathBuf) -> types::PathBufResult {
    if fs::metadata(&dir).is_err() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// # Returns
/// - `Err`: if the `dir` path does not exist
/// - `Vec<PathBuf>`: a list of files present, may be empty
pub fn filepaths_in(dir: PathBuf) -> types::VecPathBufResult {
    Ok(fs::read_dir(dir)?
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|t| t.file_type().is_ok_and(|f| f.is_file()))
        })
        .map(|e| e.unwrap().path())
        .collect())
}

/// Remove a string in its entirety from another string.
///
// TODO fix doc test
// ```
// let input = String::from("Lorem ipsum dolor sic amet.");
// assert_eq!(
//     util::remove_str_from_string(input, "dolor"),
//     "Lorem ipsum  sic amet."
// );
// ```
pub fn remove_str_from_string(s: String, to_remove: &str) -> String {
    let without = s.split(to_remove).fold(String::new(), |acc, s| acc + s);
    String::from(without.trim())
}

/// Remove leading and trailing brackets
// TODO fix doc test
// ```
// assert_eq!(util::remove_brackets("(official video)"), "official video");
// assert_eq!(util::remove_brackets("[hard remix]"), "hard remix");
// assert_eq!(util::remove_brackets("{instrumental}"), "instrumental");
// assert_eq!(util::remove_brackets("<remix>"), "remix");
// ```
pub fn remove_brackets(s: &str) -> String {
    let s = s.trim();
    let mut result = String::from(s);
    if s.starts_with(&['(', '[', '{', '<', '【']) {
        result.remove(0);
    }
    if s.ends_with(&[')', ']', '}', '>', '】']) {
        result.pop();
    }
    result
}
