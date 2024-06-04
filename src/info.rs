use crate::{types, Config};
use std::fs;

/// Show the library's status and discovered config files.
pub fn show(config: &Config) -> types::UnitResult {
    print!("\n  {}", config.library.as_ref().unwrap());
    if let Some(desc) = &config.lib_desc {
        println!(": {}\n", desc);
    } else {
        println!();
    }

    let input_path = config.input_path.as_ref().unwrap();
    if fs::metadata(input_path).is_ok() {
        print!("  > input.txt : ");
        let inputs = fs::read_to_string(input_path)?;
        if inputs.is_empty() {
            println!("Nothing to download");
        } else {
            println!("{} to download", inputs.lines().count());
        }
    }
    if fs::metadata(config.lib_conf_path.as_ref().unwrap()).is_ok() {
        println!("  > lib.conf");
    }
    if fs::metadata(config.yt_dlp_conf_path.as_ref().unwrap()).is_ok() {
        println!("  > yt-dlp.conf");
    }

    println!();
    Ok(())
}

/// Print the list of libraries discovered in the tapeworm config directory.
pub fn list() -> types::UnitResult {
    if let Ok(libraries) = fs::read_dir(dirs::config_dir().unwrap().join("tapeworm")) {
        for lib in libraries {
            let lib = lib?;
            if lib.path().is_dir() {
                println!("{}", lib.file_name().to_str().unwrap());
            }
        }
    }
    Ok(())
}

pub fn help() {
    println!(
        "\
tapeworm - A scraper and downloader written in Rust

COMMANDS
    A command may take options. If it does, the GENERAL OPTIONS also apply.

    help, h, -h, --help
        Show this help message

    list, ls, l
        List all libraries

    LIBRARY
        Show information about the LIBRARY

    LIBRARY add TERM|URL [TERM|URL...]
        Add TERMs and/or URLs to the LIBRARY. TERMs are added as YouTube search queries.
        A URL is simply added, unless it points to a Spotify playlist.
        In this case, it will be scraped, and the found songs are added as YouTube search queries.
        This is because of Spotify DRM restrictions.

        Note that YouTube search queries can be downloaded by yt-dlp.

        NB: if LIBRARY does not exist, it will be created.

    LIBRARY download [OPTIONS]
        Given the inputs in ~/.config/tapeworm/LIBRARY/input.txt,
        scrape any queries and download all (scraped) URLs,
        using the config in ~/.config/tapeworm/LIBRARY/yt-dlp.conf

        OPTIONS
        -c      Clear the input file after scraping

    LIBRARY tag [OPTIONS]
        Tag all downloaded files in the directory specified by INPUT_DIR

        OPTIONS
        -i      Set the INPUT_DIR, required if not set in lib.conf
        -t      Automatically write discovered tags

    LIBRARY deposit [OPTIONS]
        Move downloaded files to the directory specified by TARGET_DIR

        OPTIONS
        -d MODE
            Requires -o. Deposit files into an organized manner into the TARGET_DIR.
            MODE is one of the following:
            - \"A-Z\": Sort into alphabetic subfolders, and possibly ARTIST and ALBUM subfolders
            - \"DATE\": Sort into YYYY/MM subfolders
            - \"DROP\": Drop files directly in TARGET_DIR

        -i
            Set the INPUT_DIR, required if not set in lib.conf

        -o
            Set the TARGET_DIR (output directory), requires -i

    LIBRARY process [OPTIONS]
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
    tapeworm LIBRARY add song  # records 'ytsearch:song'
    tapeworm LIBRARY add \"the artist - a song\"  # records 'ytsearch:the artist - a song'
    tapeworm LIBRARY add https://youtube.com/watch?v=123

    # Download, tag, and organize all
    tapeworm LIBRARY download
    tapeworm LIBRARY tag
    tapeworm LIBRARY deposit -d A-Z

    # Alternatively, using process steps
    tapeworm LIBRARY process -s download,tag,deposit -d A-Z
"
    );
}
