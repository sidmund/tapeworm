//! Add inputs to the library.

use crate::{scrape, types, util, Config};
use url::Url;

/// Attempts to append all terms to the input file.
/// The library folder and input file are created if they do not exist.
pub fn run(config: &Config) -> types::UnitResult {
    util::guarantee_dir_path(config.lib_path.clone().unwrap())?;

    let terms = config.terms.as_ref().unwrap().iter().map(|s| s.to_string());
    let contents = format!("{}\n", parse(terms.collect()).join("\n"));
    util::append(&config.input_path.clone().unwrap(), contents)?;

    Ok(())
}

fn parse(terms: Vec<String>) -> Vec<String> {
    let mut inputs: Vec<String> = Vec::new();
    for term in terms {
        if let Ok(url) = Url::parse(&term) {
            inputs.extend(scrape(url));
        } else {
            inputs.push(format!("ytsearch:{}", term));
        }
    }
    inputs
}

/// If `url` is scrapeable, return a list of scraped queries from that page.
/// Otherwise, return `url` as a single item in the list.
fn scrape(url: Url) -> Vec<String> {
    let mut results = Vec::new();
    match url.host_str() {
        Some("open.spotify.com") if url.path().starts_with("/playlist") => {
            match scrape::spotify_playlist(url.as_str()) {
                Ok(list) => list.iter().for_each(|query| {
                    results.push(format!("ytsearch:{}", query));
                }),
                Err(e) => println!("Error scraping {}: {}\nSkipping...", url.as_str(), e),
            }
        }
        _ => results.push(url.to_string()),
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_terms() {
        let terms = vec![String::from("Darude"), String::from("Sandstorm")];
        assert_eq!(parse(terms), vec!["ytsearch:Darude", "ytsearch:Sandstorm"]);

        let terms = vec![String::from("Darude Sandstorm")];
        assert_eq!(parse(terms), vec!["ytsearch:Darude Sandstorm"]);
    }

    #[test]
    fn parses_urls() {
        let terms = vec![
            String::from("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            String::from("https://www.youtube.com/watch?v=y6120QOlsfU"),
        ];
        assert_eq!(parse(terms.clone()), terms);
    }

    #[test]
    fn parses_terms_and_urls() {
        let terms = vec![
            String::from("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            String::from("Darude Sandstorm"),
            String::from("Rickroll"),
            String::from("https://www.youtube.com/watch?v=y6120QOlsfU"),
        ];
        assert_eq!(
            parse(terms),
            vec![
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                "ytsearch:Darude Sandstorm",
                "ytsearch:Rickroll",
                "https://www.youtube.com/watch?v=y6120QOlsfU"
            ]
        );
    }
}
