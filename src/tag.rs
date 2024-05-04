use crate::Config;
use std::fs;
use std::path::PathBuf;

type UnitResult = Result<(), Box<dyn std::error::Error>>;

pub fn tag(config: &Config) -> UnitResult {
    if !config.enable_tagging {
        return Ok(());
    } else if config.yt_dlp_output_dir.is_none() {
        return Err("'YT_DLP_OUTPUT_DIR' must be set when tagging is enabled. See 'help'".into());
    }

    let downloads =
        PathBuf::from(config.lib_path.clone()).join(config.yt_dlp_output_dir.clone().unwrap());
    for entry in fs::read_dir(downloads)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            continue;
        }

        println!("{}", entry.file_name().to_str().unwrap());
    }

    Ok(())
}
