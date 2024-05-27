//! Informational functionality.

use crate::{types, Config};
use std::{fs, path::PathBuf};

/// Show the library's discovered config files.
pub fn show(config: &Config) -> types::UnitResult {
    let lib = config.library.clone().unwrap();
    let desc = config.lib_desc.clone().unwrap_or(String::from(""));
    println!("\n  {}: {}\n", lib, desc);

    let input_path = config.input_path.clone().unwrap();
    if fs::metadata(&input_path).is_ok() {
        print!("  > input.txt : ");
        let inputs = fs::read_to_string(&input_path)?;
        if inputs.is_empty() {
            println!("Nothing to download");
        } else {
            println!("{} to download", inputs.lines().count());
        }
    }

    if fs::metadata(&config.lib_conf_path.clone().unwrap()).is_ok() {
        println!("  > lib.conf");
    }

    if fs::metadata(&config.yt_dlp_conf_path.clone().unwrap()).is_ok() {
        println!("  > yt-dlp.conf");
    }

    println!();
    Ok(())
}

/// Print the list of libraries discovered in the tapeworm config directory.
pub fn list() -> types::UnitResult {
    let conf_path = PathBuf::from(dirs::config_dir().unwrap()).join("tapeworm");
    if let Ok(libraries) = fs::read_dir(&conf_path) {
        libraries
            .map(|l| l.unwrap())
            .filter(|l| l.path().is_dir())
            .for_each(|l| println!("{}", l.file_name().to_str().unwrap()));
    }

    Ok(())
}

pub fn help() {
    println!(
        "\
tapeworm - A scraper and downloader written in Rust

COMMANDS
    A command may take options. If it does, the GENERAL OPTIONS also apply.

    help
        Show this help message

    list
        List all libraries

    show LIBRARY
        Show information about the LIBRARY

    add LIBRARY TERM|URL [TERM|URL...]
        Add TERMs and/or URLs to the LIBRARY. TERMs are added as YouTube search queries.
        A URL is simply added, unless it points to a Spotify playlist.
        In this case, it will be scraped, and the found songs are added as YouTube search queries.
        This is because of Spotify DRM restrictions.

        Note that YouTube search queries can be downloaded by yt-dlp.

        NB: if LIBRARY does not exist, it will be created.

    download LIBRARY [OPTIONS]
        Given the inputs in ~/.config/tapeworm/LIBRARY/input.txt,
        scrape any queries and download all (scraped) URLs,
        using the config in ~/.config/tapeworm/LIBRARY/yt-dlp.conf

        OPTIONS
        -c      Clear the input file after scraping

    tag LIBRARY [OPTIONS]
        Tag all downloaded files in the directory specified by INPUT_DIR

        OPTIONS
        -i      Set the INPUT_DIR, required if not set in lib.conf

    deposit LIBRARY [OPTIONS]
        Move downloaded files to the directory specified by TARGET_DIR

        OPTIONS
        -d MODE
            Requires -o. Deposit files into an organized manner into the TARGET_DIR.
            MODE is one of the following:
            - \"A-Z\": Sort into alphabetic subfolders, and possibly ARTIST and ALBUM subfolders

        -i
            Set the INPUT_DIR, required if not set in lib.conf

        -o
            Set the TARGET_DIR (output directory), requires -i

    process LIBRARY [OPTIONS]
        Process LIBRARY as specified by `STEPS`. Any options from `download`, `tag`, `deposit` are valid here

        OPTIONS
        -s      Set the processing steps (commands) to run on the library as a
                comma-separated list, required if not set in lib.conf

GENERAL OPTIONS
    The options from ~/.config/tapeworm/LIBRARY/lib.conf are loaded first.
    Setting a CLI option will override its value in the lib.conf file, if present.

    -v      Verbosely show what is being processed

EXAMPLE
    # Create the library by recording a query
    tapeworm add LIBRARY song  # records 'ytsearch:song'
    tapeworm add LIBRARY \"the artist - a song\"  # records 'ytsearch:the artist - a song'
    tapeworm add LIBRARY https://youtube.com/watch?v=123

    # Download, tag, and organize all
    tapeworm download LIBRARY
    tapeworm tag LIBRARY
    tapeworm deposit LIBRARY -d A-Z

    # Alternatively, using process steps
    tapeworm process LIBRARY -s download,tag,deposit -d A-Z
"
    );
}
