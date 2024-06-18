use crate::{types, util, Config};
use std::fs;
use std::io::{self, Write};
use tabwriter::TabWriter;

/// Show the library's status and discovered config files.
pub fn show(config: &Config) -> types::UnitResult {
    println!(
        "\n  Library: {}",
        config.lib_path.as_ref().unwrap().display()
    );
    if let Some(alias) = &config.lib_alias {
        println!("  > Alias: {}", alias);
    }
    if let Some(desc) = &config.lib_desc {
        println!("  > Description: {}", desc);
    }
    println!();

    let input_dir = config.input_dir.as_ref().unwrap();
    println!("  Input folder: {}", input_dir.display());
    let mut n = 0;
    if let Ok(files) = util::filepaths_in(input_dir) {
        n = files.len()
    }
    println!("  > {} files", n);
    println!();

    let output_dir = config.target_dir.as_ref().unwrap();
    println!("  Target folder: {}", output_dir.display());
    println!();

    println!("  Configuration files:");
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

/// Print the list of aliases.
pub fn list(config: &Config) {
    let mut tw = TabWriter::new(io::stdout());
    writeln!(&mut tw, "ALIAS\tLIBRARY PATH").unwrap();
    for (alias, path) in &config.aliases {
        writeln!(&mut tw, "{}\t{}", alias, path.display()).unwrap();
    }
    tw.flush().unwrap();
}

pub fn help() {
    println!(
        "\
tapeworm - A scraper and downloader written in Rust

COMMANDS
    If a command takes [OPTIONS] (sic), the GENERAL OPTIONS also apply.
    Note that LIBRARY refers to either the library path or its alias.

    help, h, -h, --help
        Show this help message

    list, ls, l
        List all library aliases

    LIBRARY
        Show information about the LIBRARY

    LIBRARY add TERM|URL [TERM|URL...]
        Add TERMs and/or URLs to the LIBRARY. TERMs are added as YouTube search queries. A URL is simply added, unless it points to a Spotify playlist. In this case, it will be scraped, and the found songs are added as YouTube search queries. This is because of Spotify DRM restrictions.

        Note that YouTube search queries can be downloaded by yt-dlp.

    LIBRARY download [OPTIONS]
        Given the inputs in ~/.config/tapeworm/LIBRARY/input.txt, scrape any queries and download all (scraped) URLs, using the config in ~/.config/tapeworm/LIBRARY/yt-dlp.conf

        OPTIONS
        -c          Clear the input file after scraping
        -a          Automatically keep downloads (no confirmation prompt)

    LIBRARY tag [OPTIONS]
        Tag all files in the input directory

        OPTIONS
        -i IN       What directory to look in for files to tag. By default, this is the `.tapeworm/tmp` folder
        -t          Automatically write discovered tags (no confirmation prompt and no edit possibility)

    LIBRARY deposit [OPTIONS]
        Move downloaded files to the directory specified by TARGET_DIR

        OPTIONS
        -d MODE     Organize files into the output directory. MODE is one of the following:
                    - \"A-Z\": Sort into alphabetic subfolders, and possibly ARTIST and ALBUM subfolders
                    - \"DATE\": Sort into YYYY/MM subfolders
                    - \"DROP\": Drop files directly in TARGET_DIR
        -i IN       What directory to find files in. By default, this is the `.tapeworm/tmp` folder
        -o OUT      What directory to move files to. By default, this is the library root folder

    LIBRARY process [OPTIONS]
        Process LIBRARY as specified by `STEPS`. Any options from `download`, `tag`, `deposit` are valid here

        OPTIONS
        -s          Set the processing steps (commands) to run on the library as a comma-separated list, required if not set in lib.conf

    LIBRARY clean [OPTIONS]
        Removes empty folders

    alias ALIAS [OPTION]
        Configure the ALIAS for a library. With an alias, any library command can be specified with the alias instead of the full library path. Without an option, this command will show the library path for ALIAS

        OPTION
        -p PATH     Add or overwrite ALIAS with the given library PATH
        -r          Remove this ALIAS

GENERAL OPTIONS
    The options from path/to/library/.tapeworm/lib.conf are loaded first.
    Setting a CLI option will override its value in the lib.conf file, if present.

    -v      Verbosely show what is being processed

EXAMPLE
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
