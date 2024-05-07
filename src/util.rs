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
