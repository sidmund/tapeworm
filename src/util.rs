use crate::types;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

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
