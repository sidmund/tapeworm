use crate::Config;
use regex::Regex;
use std::collections::HashMap;
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

        let filename = entry.file_name();
        let filename = filename.to_str().unwrap();
        parse_filename(filename);
    }

    Ok(())
}

fn parse_filename(filename: &str) -> Option<HashMap<&str, String>> {
    println!("Parsing: {}", filename);

    let mut tags: HashMap<&str, String> = HashMap::new();

    let (mut filename, extension) = filename.split_at(filename.rfind(".").unwrap());
    tags.insert("extension", String::from(&extension[1..]));

    for delim in "-_~|".chars() {
        if let Some((author, title)) = filename.split_once(delim) {
            let author: String = author
                .split(&['&', ','])
                .map(|s| s.trim().to_string())
                .reduce(|acc, s| format!("{}&{}", acc, s))
                .unwrap();
            tags.insert("author", author);

            let title = title.trim();
            tags.insert("title", String::from(title));

            filename = title;
            break;
        }
    }

    let re = Regex::new(
        r"(?xi)
        (?<year>\(\d{4}\)|\d{4}) |
        ([\[({<](?<remix>[^\[\](){}<>]*remix[^\[\](){}<>]*)[\])}>]) |
        ([\[({<](?<bootleg>[^\[\](){}<>]*bootleg[^\[\](){}<>]*)[\])}>])
        ",
    )
    .unwrap();

    for caps in re.captures_iter(filename) {
        println!("Captures: {:?}", caps);

        if let Some(year) = caps.name("year") {
            let mut year = year.as_str();
            if year.starts_with("(") {
                year = &year[1..5];
            }
            tags.insert("year", String::from(year));
        }

        if let Some(remix) = caps.name("remix") {
            tags.insert("remix", String::from(remix.as_str()));
        }

        if let Some(bootleg) = caps.name("bootleg") {
            tags.insert("bootleg", String::from(bootleg.as_str()));
        }
    }

    println!("Got tags: {:?}", tags);

    Some(tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO add test cases as I encounter them IRL and make the parser more robust
    // TODO test case insensitivity

    #[test]
    fn test_year() {
        let tags = parse_filename("Band - Song (2024).mp3").unwrap();
        assert_eq!(tags["year"], "2024");

        let tags = parse_filename("Band - Song 2024.mp3").unwrap();
        assert_eq!(tags["year"], "2024");

        let tags = parse_filename("Band - Song.mp3").unwrap();
        assert_eq!(tags.get("year"), None);
    }
}
