use crate::types;
use crate::Config;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn tag(config: &Config) -> types::UnitResult {
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
        // TODO instead of filename, use the TITLE metadata
        build_tags(filename, config.verbose);
    }

    Ok(())
}

/// Attempt to extract tags from the title metadata.
///
/// Returns None if no tags could be extracted.
/// Otherwise, returns a HashMap with all found tags, a subset of:
/// - author: a '&' separated list of authors
/// - title: the title after removing other/spurious information
/// - year
/// - remix: (re)mixes, remasters, bootlegs, instrumental are treated as 'remix'
fn build_tags(meta_title: &str, verbose: bool) -> Option<HashMap<&str, String>> {
    if verbose {
        println!("Parsing: {}", meta_title);
    }

    let mut tags: HashMap<&str, String> = HashMap::new();

    let mut meta_title = meta_title;
    let mut title = None;

    for delim in "-_~｜".chars() {
        if let Some((author, full_title)) = meta_title.split_once(delim) {
            let author = Regex::new(r"(featuring|feat\.?|ft\.?|&|,)")
                .unwrap()
                .split(author)
                .map(|s| s.trim().to_string())
                .reduce(|acc, s| format!("{}&{}", acc, s))
                .unwrap();
            tags.insert("author", author);

            let full_title = full_title.trim();
            title = Some(full_title.to_string());
            meta_title = full_title;
            break;
        }
    }

    let re = Regex::new(
        r"(?xi)
        (?<year>\(\d{4}\)|\d{4}) |
        (?<remix>[\[({<][^\[\](){}<>]*((re)?mix|remaster|bootleg|instrumental)[^\[\](){}<>]*[\])}>]) |
        (?<strip>[\[({<][^\[\](){}<>]*(official video)[^\[\](){}<>]*[\])}>])
        ",
    );

    for caps in re.unwrap().captures_iter(meta_title) {
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

        if let Some(strip) = caps.name("strip") {
            if let Some(t) = title {
                title = Some(remove_str_from_string(t, strip.as_str()));
            }
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

    #[test]
    fn test_spacing() {
        let songs = ["Band - Song", "Band- Song", "Band -Song", "Band-Song"];
        for song in songs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], "Band");
            assert_eq!(tags["title"], "Song");
        }
    }

    #[test]
    fn test_author() {
        let tags = build_tags("Artist & Band - Song", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");

        let tags = build_tags("Artist, Other & Another - Song", true).unwrap();
        assert_eq!(tags["author"], "Artist&Other&Another");

        let tags = build_tags("Artist ft. Other - Song", true).unwrap();
        assert_eq!(tags["author"], "Artist&Other");

        let tags = build_tags("Artist & Band feat. Other - Song", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band&Other");
    }

    #[test]
    fn test_year() {
        let tags = build_tags("Band - Song (2024)", true).unwrap();
        assert_eq!(tags["author"], "Band");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["year"], "2024");

        let tags = build_tags("Band - Song 2024", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["year"], "2024");

        let tags = build_tags("Band - Song", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("year"), None);
    }

    #[test]
    fn test_remix() {
        let tags = build_tags("Artist - Song [Club Remix]", true).unwrap();
        assert_eq!(tags["author"], "Artist");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "Club Remix");

        let tags = build_tags("Artist- Song (HQ REMASTER)", true).unwrap();
        assert_eq!(tags["author"], "Artist");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "HQ REMASTER");

        let tags = build_tags("Artist & Band - Song (radio mix) 2003", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "radio mix");
        assert_eq!(tags["year"], "2003");

        let tags = build_tags("Artist - Song [Instrumental]", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "Instrumental");
    }
}
