//! Informational functions.

use crate::types;
use std::fs;
use std::path::PathBuf;

pub fn list() -> types::UnitResult {
    let conf_path = PathBuf::from(dirs::config_dir().unwrap()).join("tapeworm");
    let libraries = fs::read_dir(&conf_path);
    if libraries.is_err() {
        return Ok(()); // No need to fail when no libraries are present
    }

    for library in libraries.unwrap() {
        let library = library?;
        if library.path().is_dir() {
            println!("{}", library.file_name().to_str().unwrap());
        }
    }

    Ok(())
}

pub fn help() {
    println!(
        "\
tapeworm - A scraper and downloader written in Rust

COMMANDS
    help
        Show this help message

    list
        List all libraries

    add LIBRARY URL [URL...]
        Add URLs to the library. If the URL points to a Spotify playlist,
        it will be scraped, and the found songs are added as YouTube search queries.
        This is because of Spotify DRM restrictions.

        If LIBRARY does not exist, it will be created.

    add LIBRARY TERM [TERM...]
        Combine all terms into a single search query and add it to the library.
        NB: when invoking 'download', a YouTube video will be found for the query.

        If LIBRARY does not exist, it will be created.

    download LIBRARY [OPTIONS]
        Given the inputs in ~/.config/tapeworm/LIBRARY/input.txt,
        scrape any queries and download all (scraped) URLs,
        using the config in ~/.config/tapeworm/LIBRARY/yt-dlp.conf

DOWNLOAD OPTIONS
    The options from ~/.config/tapeworm/LIBRARY/lib.conf are loaded first.
    Setting a CLI option will override its value in the lib.conf file, if present.

    -c      Clear the input file after scraping
    -d      Deposit downloaded files in organized subfolders of TARGET_DIR, requires -o
    -o      Set the TARGET_DIR, requires -y
    -t      Enable tagging, requires -y
    -v      Verbosely show what is being processed
    -y      Set the YT_DLP_OUTPUT_DIR, required by -o and -t

EXAMPLE
    # Create the library by recording the first query
    tapeworm add LIBRARY the artist - a song  # records 'the artist - a song'

    # Add a URL
    tapeworm add LIBRARY https://youtube.com/watch?v=123

    # Scrape/download all
    tapeworm download LIBRARY
"
    );
}
