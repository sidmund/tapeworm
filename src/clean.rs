use crate::{types, Config};
use std::fs::{self, DirEntry};
use std::path::PathBuf;

pub fn run(config: &Config) -> types::UnitResult {
    remove_empty_folders(config.lib_path.as_ref().unwrap(), 0, config.verbose)
}

/// Remove empty folders, except for ".tapeworm".
///
/// # Parameters
/// - `root`: The folder to start from
/// - `depth`: The current depth in the folder tree, must start at 0
/// - `verbose`: Whether to print removed directories
fn remove_empty_folders(root: &PathBuf, depth: i8, verbose: bool) -> types::UnitResult {
    let entries = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .collect::<Vec<DirEntry>>();
    if entries.is_empty() {
        if verbose {
            println!("Removing empty folder: {}", root.display());
        }
        fs::remove_dir(root)?;
        if depth > 1 {
            // Go back up (if not at the initial root) to check if the parent has now become empty
            remove_empty_folders(&root.parent().unwrap().to_path_buf(), depth - 1, verbose)?;
        }
        return Ok(());
    }

    for entry in entries {
        if entry.file_name() == ".tapeworm" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            remove_empty_folders(&entry.path(), depth + 1, verbose)?;
        }
    }
    Ok(())
}
