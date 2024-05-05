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

    let downloads = PathBuf::from(config.lib_path.clone().unwrap())
        .join(config.yt_dlp_output_dir.clone().unwrap());
    for entry in fs::read_dir(downloads)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            continue;
        }

        let filename = entry.file_name();
        let filename = filename.to_str().unwrap();
        build_tags(filename, config.verbose);
    }

    Ok(())
}

/// Attempt to extract tags from filename.
///
/// Returns None if no tags could be extracted.
/// Otherwise, returns a HashMap with all found tags, a subset of:
/// - author: a '&' separated list of authors
/// - title
/// - year
/// - remix: (re)mixes, remasters, bootlegs are treated as 'remix'
/// - extension
fn build_tags(filename: &str, verbose: bool) -> Option<HashMap<&str, String>> {
    if verbose {
        println!("Parsing: {}", filename);
    }

    let mut tags: HashMap<&str, String> = HashMap::new();

    let (mut filename, extension) = filename.split_at(filename.rfind(".").unwrap());
    tags.insert("extension", String::from(&extension[1..]));

    let mut title = None;

    for delim in "-_~|".chars() {
        if let Some((author, full_title)) = filename.split_once(delim) {
            let author = Regex::new(r"(featuring|feat\.?|ft\.?|&|,)")
                .unwrap()
                .split(author)
                .map(|s| s.trim().to_string())
                .reduce(|acc, s| format!("{}&{}", acc, s))
                .unwrap();
            tags.insert("author", author);

            let full_title = full_title.trim();
            title = Some(full_title.to_string());
            filename = full_title;
            break;
        }
    }

    let re = Regex::new(
        r"(?xi)
        (?<year>\(\d{4}\)|\d{4}) |
        (?<remix>[\[({<][^\[\](){}<>]*((re)?mix|remaster|bootleg)[^\[\](){}<>]*[\])}>])
        ",
    );

    for caps in re.unwrap().captures_iter(filename) {
        if verbose {
            println!("Captures: {:?}", caps);
        }

        if let Some(year) = caps.name("year") {
            let year = year.as_str();
            if let Some(t) = title {
                title = Some(remove_str_from_string(t, year));
            }
            tags.insert("year", remove_brackets(year));
        }

        if let Some(remix) = caps.name("remix") {
            let remix = remix.as_str();
            if let Some(t) = title {
                title = Some(remove_str_from_string(t, remix));
            }
            tags.insert("remix", remove_brackets(remix));
        }
    }

    if let Some(title) = title {
        tags.insert("title", title);
    }

    println!("Got tags: {:?}", tags);

    Some(tags)
}

fn remove_str_from_string(s: String, to_remove: &str) -> String {
    let without = s.split(to_remove).fold(String::new(), |acc, s| acc + s);
    String::from(without.trim())
}

/// Remove leading and trailing brackets
fn remove_brackets(s: &str) -> String {
    let s = s.trim();
    let mut result = String::from(s);
    if s.starts_with(&['(', '[', '{', '<']) {
        result.remove(0);
    }
    if s.ends_with(&[')', ']', '}', '>']) {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO Add test cases as you encounter them IRL

    #[test]
    fn test_author() {
        let tags = build_tags("Band - Song.mp3", true).unwrap();
        assert_eq!(tags["author"], "Band");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["extension"], "mp3");

        let tags = build_tags("Artist & Band - Song.mp3", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");

        let tags = build_tags("Artist, Other & Another - Song.mp3", true).unwrap();
        assert_eq!(tags["author"], "Artist&Other&Another");

        let tags = build_tags("Artist ft. Other - Song.mp3", true).unwrap();
        assert_eq!(tags["author"], "Artist&Other");

        let tags = build_tags("Artist & Band feat. Other - Song.mp3", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band&Other");
    }

    #[test]
    fn test_year() {
        let tags = build_tags("Band - Song (2024).mp3", true).unwrap();
        assert_eq!(tags["author"], "Band");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["year"], "2024");
        assert_eq!(tags["extension"], "mp3");

        let tags = build_tags("Band - Song 2024.mp3", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["year"], "2024");

        let tags = build_tags("Band - Song.mp3", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("year"), None);
    }

    #[test]
    fn test_remix() {
        let tags = build_tags("Artist - Song [Club Remix].mp3", true).unwrap();
        assert_eq!(tags["author"], "Artist");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "Club Remix");
        assert_eq!(tags["extension"], "mp3");

        let tags = build_tags("Artist - Song (HQ REMASTER).mp3", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "HQ REMASTER");

        let tags = build_tags("Artist & Band - Song (radio mix) 2003.mp3", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "radio mix");
        assert_eq!(tags["year"], "2003");
    }
}
