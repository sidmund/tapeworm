//! Add inputs to the library.

use crate::scrape;
use crate::types;
use crate::util;
use crate::Config;
use url::Url;

/// Attempts to append all terms to the input file.
/// The library folder and input file are created if they do not exist.
pub fn add(config: &Config) -> types::UnitResult {
    util::guarantee_dir_path(config.lib_path.clone().unwrap())?;

    let mut inputs: Vec<String> = Vec::new();
    for term in config.terms.as_ref().unwrap().iter().map(|s| s.to_string()) {
        if let Ok(url) = Url::parse(&term) {
            let host = url.host_str();
            let path = url.path();

            if host == Some("open.spotify.com") && path.starts_with("/playlist") {
                let scraped = scrape::spotify_playlist(config, &term);
                if scraped.is_err() {
                    println!("Error scraping {}: {}", term, scraped.unwrap_err());
                    println!("Skipping...");
                } else {
                    for scraped in scraped.unwrap() {
                        inputs.push(format!("ytsearch:\"{}\"", scraped));
                    }
                }
                continue;
            }
        }

        inputs.push(term);
    }

    let contents = format!("{}\n", inputs.join("\n"));
    util::append(&config.input_path.clone().unwrap(), contents)?;

    Ok(())
}
