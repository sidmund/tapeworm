//! This module provides functionality for extracting tags from a filename.

use crate::types;
use crate::util;
use crate::Config;
use audiotags::Tag;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// For each downloaded file, use its "title" metadata tag to extract more tags. If this tag is not
/// present in the file, it will not be affected.
///
/// Titles generally contain extra information, e.g. "Artist ft. Band - Song (2024) [Remix]"
/// Information such as collaborating artists, year, remix, etc. are extracted.
pub fn tag(config: &Config) -> types::UnitResult {
    if !config.enable_tagging {
        return Ok(());
    } else if config.yt_dlp_output_dir.is_none() {
        return Err("'YT_DLP_OUTPUT_DIR' must be set when tagging is enabled. See 'help'".into());
    }

    println!("\nTAGGING FILES...");

    let downloads = PathBuf::from(config.lib_path.clone().unwrap())
        .join(config.yt_dlp_output_dir.clone().unwrap());
    let downloads: Vec<PathBuf> = fs::read_dir(downloads)?
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|t| t.file_type().is_ok_and(|f| f.is_file()))
        })
        .map(|e| e.unwrap().path())
        .collect();
    let total = downloads.len();

    for (i, entry) in downloads.iter().enumerate() {
        let filename = entry.file_name().unwrap().to_owned().into_string().unwrap();
        println!("Tagging {} of {}: {}", i + 1, total, filename);

        let mut entry_tag = Tag::new().read_from_path(entry)?;
        let title = entry_tag.title();
        if title.is_none() {
            continue;
        }

        let title = String::from(title.unwrap());
        let tags = build_tags(title.as_str(), config.verbose);
        if tags.is_none() {
            continue;
        }
        let tags = tags.unwrap();

        let mut year = None;
        if let Some(y) = tags.get("year") {
            if let Ok(y) = y.parse::<i32>() {
                year = Some(y);
            } else {
                eprintln!("year is not a number: {}, discarding", y);
            }
        }

        let mut title = if let Some(title) = tags.get("title") {
            Some(title.to_owned())
        } else {
            None
        };

        let mut artist = None;
        if let Some(author) = tags.get("author") {
            let mut artists = author.split("&");

            // First artist is seen as main
            if let Some(a) = artists.next() {
                artist = Some(a);
            }

            let mut feat = String::new();
            while let Some(a) = artists.next() {
                feat = format!("{}, {}", feat, a);
            }
            if let Some(i) = feat.rfind(',') {
                feat.replace_range(i..=i, " &");
            }
            if !feat.is_empty() {
                title = Some(format!("{} ({})", title.unwrap_or(String::new()), feat));
            }
        }

        if let Some(remix) = tags.get("remix") {
            title = Some(format!("{} [{}]", title.unwrap_or(String::new()), remix));
        }

        let new_filename = if let Some(artist) = artist.clone() {
            if let Some(title) = title.clone() {
                format!("{} - {}.mp3", artist, title)
            } else {
                format!("{}.mp3", artist)
            }
        } else if let Some(title) = title.clone() {
            if let Some(tag_artist) = entry_tag.artist() {
                // When filename led to only title being extracted, but the artist tag was set by
                // yt-dlp, e.g. "Song.mp3" only gives tags "title: Song" but yt-dlp set the artist
                format!("{} - {}.mp3", tag_artist, title)
            } else {
                format!("{}.mp3", title)
            }
        } else {
            filename.clone()
        };

        println!("Proposed changes:");
        print_proposal("FILENAME", Some(&filename), Some(&new_filename));
        println!("Tags:");
        print_proposal("ARTIST", entry_tag.artist(), artist);
        print_proposal("ALBUM_ARTIST", entry_tag.album_artist(), artist);
        print_proposal(
            "TITLE",
            entry_tag.title(),
            title.as_ref().map(|s| s.as_str()),
        );
        print_proposal("YEAR", entry_tag.year(), year);

        if util::confirm("Accept these changes?", true)? {
            // Write tags
            if let Some(artist) = artist {
                entry_tag.set_artist(&artist);
                entry_tag.set_album_artist(&artist);
            }
            if let Some(title) = title {
                entry_tag.set_title(title.as_str());
            }
            if let Some(year) = year {
                entry_tag.set_year(year);
            }
            entry_tag.write_to_path(entry.to_str().unwrap())?;

            if new_filename != filename {
                fs::rename(entry, entry.with_file_name(new_filename))?;
            }
        }
    }

    Ok(())
}

fn print_proposal<T>(name: &str, old: Option<T>, new: Option<T>)
where
    T: std::fmt::Display + PartialEq,
{
    if old.is_none() {
        if new.is_none() {
            println!("{:<15} N/A", name);
        } else {
            println!("{:<15} N/A -> {}", name, new.unwrap());
        }
    } else {
        let old = old.unwrap();
        if new.is_none() || new.as_ref().is_some_and(|x| *x == old) {
            println!("{:<15} {} -> unchanged", name, old);
        } else {
            println!("{:<15} {} -> {}", name, old, new.unwrap());
        }
    }
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
    let mut title = meta_title.to_string();

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
            title = full_title.to_string();
            meta_title = full_title;
            break;
        }
    }

    let re = Regex::new(
        r"(?xi)
        (?<year>\(\d{4}\)|\d{4}) |
        (?<remix>[\[({<][^\[\](){}<>]*((re)?mix|remaster|bootleg|instrumental)[^\[\](){}<>]*[\])}>]) |
        (?<strip>[\[({<][^\[\](){}<>]*((official\s)?(music\s)?video|m/?v|hq|hd)[^\[\](){}<>]*[\])}>])
        ",
    );

    for caps in re.unwrap().captures_iter(meta_title) {
        if verbose {
            println!("Captures: {:?}", caps);
        }

        if let Some(year) = caps.name("year") {
            let year = year.as_str();
            title = util::remove_str_from_string(title, year);
            tags.insert("year", util::remove_brackets(year));
        }

        if let Some(remix) = caps.name("remix") {
            let remix = remix.as_str();
            title = util::remove_str_from_string(title, remix);
            tags.insert("remix", util::remove_brackets(remix));
        }

        if let Some(strip) = caps.name("strip") {
            title = util::remove_str_from_string(title, strip.as_str());
        }
    }

    tags.insert("title", title);

    if verbose {
        println!("Got tags: {:?}", tags);
    }

    Some(tags)
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

    #[test]
    fn test_strip() {
        let tags = build_tags("Artist - Song [HQ]", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("remix"), None);

        let tags = build_tags("Artist - Song [HD]", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("remix"), None);

        let tags = build_tags("Artist - Song [M/V]", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("remix"), None);

        let tags = build_tags("Artist - Song (Official Music Video)", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("remix"), None);

        let tags = build_tags("Artist - Song (Official Video)", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("remix"), None);

        let tags = build_tags("Artist - Song (Music Video)", true).unwrap();
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags.get("remix"), None);
    }
}
