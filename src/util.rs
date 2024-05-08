//! Utility functions.

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

pub fn remove_str_from_string(s: String, to_remove: &str) -> String {
    let without = s.split(to_remove).fold(String::new(), |acc, s| acc + s);
    String::from(without.trim())
}

/// Remove leading and trailing brackets
pub fn remove_brackets(s: &str) -> String {
    let s = s.trim();
    let mut result = String::from(s);
    if s.starts_with(&['(', '[', '{', '<']) {
        result.remove(0);
    }
    if s.ends_with(&[')', ']', '}', '>']) {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_removal() {
        assert_eq!(
            remove_str_from_string(String::from("Lorem ipsum dolor sic amet."), "dolor"),
            "Lorem ipsum  sic amet."
        );
    }

    #[test]
    fn test_remove_brackets() {
        assert_eq!(remove_brackets("(official video)"), "official video");
        assert_eq!(remove_brackets("[hard remix]"), "hard remix");
        assert_eq!(remove_brackets("{instrumental}"), "instrumental");
        assert_eq!(remove_brackets("<remix>"), "remix");
    }
}
