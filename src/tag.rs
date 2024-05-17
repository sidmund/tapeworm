//! This module provides functionality for extracting tags from a filename.

use crate::types;
use crate::util;
use crate::Config;
use audiotags::Tag;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// For each downloaded file, use its "title" metadata tag to extract more tags. If this tag is not
/// present in the file, it will not be affected.
///
/// Titles generally contain extra information, e.g. "Artist ft. Band - Song (2024) [Remix]"
/// Information such as collaborating artists, year, remix, etc. are extracted.
pub fn tag(config: &Config) -> types::UnitResult {
    if config.input_dir.is_none() {
        return Err("'INPUT_DIR' must be set. See 'help'".into());
    }

    let downloads =
        PathBuf::from(config.lib_path.clone().unwrap()).join(config.input_dir.clone().unwrap());
    let downloads: Vec<PathBuf> = util::filepaths_in(downloads).unwrap_or(vec![]);
    let total = downloads.len();

    for (i, entry) in downloads.iter().enumerate() {
        let filename = entry.file_name().unwrap().to_owned().into_string().unwrap();
        println!("Tagging {} of {}: {}", i + 1, total, filename);

        let mut entry_tag = Tag::new().read_from_path(entry)?;
        let title = entry_tag.title();
        if title.is_none() {
            println!("  No 'title' tag found, skipping");
            continue;
        }

        let title = String::from(title.unwrap());
        let tags = build_tags(title.as_str(), config.verbose);
        if tags.is_none() {
            println!("  No additional tags found in title, skipping");
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

        let old_album = if let Some(a) = entry_tag.album() {
            Some(String::from(a.title))
        } else {
            None
        };
        let album = if let Some(album) = tags.get("album") {
            Some(album.to_owned())
        } else {
            None
        };

        let mut title = if let Some(title) = tags.get("title") {
            Some(title.to_owned())
        } else {
            None
        };

        let old_album_artist = if let Some(a) = entry_tag.album_artist() {
            Some(String::from(a))
        } else {
            None
        };

        let mut old_artist = None;
        let mut artist = None;
        let mut feat: HashSet<String> = HashSet::new();
        if let Some(a) = entry_tag.artist() {
            old_artist = Some(String::from(a));
            if !config.override_artist {
                let mut multiple = separate_authors(&old_artist.clone().unwrap());
                artist = Some(multiple.remove(0));
                let multiple = multiple
                    .into_iter()
                    .filter(|s| s != &artist.clone().unwrap());
                feat.extend(multiple);
            }
        }

        if let Some(author) = tags.get("author") {
            let mut multiple: Vec<String> = author.split("&").map(|s| s.to_string()).collect();
            if artist.is_none() {
                artist = Some(multiple.remove(0)); // First is treated as main artist
                let multiple = multiple
                    .into_iter()
                    .filter(|s| s != &artist.clone().unwrap());
                feat.extend(multiple);
            } else {
                feat.extend(multiple);
            }
        }

        // Modify the title so it includes the featuring artists, e.g. "(ARTIST, ARTIST & ARTIST)"
        let feat: HashSet<String> = feat.into_iter().collect(); // Remove dupes
        if let Some(mut feat) = feat.into_iter().reduce(|a, b| format!("{}, {}", a, b)) {
            if let Some(i) = feat.rfind(',') {
                feat.replace_range(i..=i, " &");
            }
            title = Some(format!("{} ({})", title.unwrap_or(String::new()), feat));
        }

        if let Some(remix) = tags.get("remix") {
            title = Some(format!("{} [{}]", title.unwrap_or(String::new()), remix));
        }

        let genre = if let Some(g) = tags.get("genre") {
            Some(g.as_str())
        } else {
            None
        };

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

        println!("\nProposed changes:");
        print_proposal("ARTIST", &old_artist, &artist);
        if old_album.is_some() || album.is_some() {
            print_proposal("ALBUM_ARTIST", &old_album_artist, &artist);
        }
        print_proposal("ALBUM", &old_album, &album);
        print_proposal(
            "TITLE",
            &entry_tag.title(),
            &title.as_ref().map(|s| s.as_str()),
        );
        print_proposal("YEAR", &entry_tag.year(), &year);
        print_proposal("GENRE", &entry_tag.genre(), &genre);
        print_proposal("FILENAME", &Some(&filename), &Some(&new_filename));

        if util::confirm("Accept these changes?", true)? {
            // Write tags
            if let Some(artist) = artist.clone() {
                entry_tag.set_artist(&artist);
            }
            if old_album.is_some() || album.is_some() {
                if let Some(artist) = artist {
                    entry_tag.set_album_artist(&artist);
                }
            }
            if let Some(album) = album {
                entry_tag.set_album_title(album.as_str());
            }
            if let Some(title) = title {
                entry_tag.set_title(title.as_str());
            }
            if let Some(year) = year {
                entry_tag.set_year(year);
            }
            if let Some(genre) = genre {
                entry_tag.set_genre(genre);
            }
            entry_tag.write_to_path(entry.to_str().unwrap())?;

            if new_filename != filename {
                fs::rename(entry, entry.with_file_name(new_filename))?;
            }
        }
    }

    Ok(())
}

fn print_proposal<T>(name: &str, old: &Option<T>, new: &Option<T>)
where
    T: std::fmt::Display + PartialEq,
{
    if old.is_none() {
        if new.is_some() {
            println!("  {:<15} N/A\n{:<20}-> {}", name, "", new.as_ref().unwrap());
        } // No need to print anything when both are none
    } else {
        let old = old.as_ref().unwrap();
        if new.is_none() || new.as_ref().is_some_and(|x| *x == *old) {
            println!("  {:<15} {}\n{:<20}(keep as is)", name, old, "");
        } else {
            let new = new.as_ref().unwrap();
            println!("  {:<15} {}\n{:<20}-> {}", name, old, "", new);
        }
    }
}

/// Separates a string like "Band ft Artist, Musician & Singer"
/// into a vector like ["Band", "Artist", "Musician", "Singer"].
fn separate_authors(s: &str) -> Vec<String> {
    let re = Regex::new(r"(?i)(\sand\s|(^|\s)featuring|(^|\s)feat\.?|(^|\s)ft\.?|(^|\s)w[⧸/]|&|,)");
    re.unwrap().split(s).map(|a| a.trim().to_string()).collect()
}

/// Attempt to split the full title into an author (left) and title (right) part.
///
/// # Returns
/// - `None`: when the title could not be split
/// - `Some(Vec<String>, String)`: a list of authors and the rest of the title
///
/// # Example
/// For the input "Band ft Artist, Musician & Singer - Song",
/// the returned authors are ["Band", "Artist", "Musician", "Singer"]
/// and the returned title is "Song".
fn from_split(full_title: &str) -> Option<(Vec<String>, String)> {
    for delim in "-_~｜".chars() {
        if let Some((authors, title)) = full_title.split_once(delim) {
            return Some((separate_authors(authors), title.trim().to_string()));
        }
    }

    None
}

/// Attempt to get the genre, artist, and title from the full title.
///
/// # Parameters
/// - `full_title`: expected to be in the format `「GENRE」[ARTIST] TITLE`
fn from_format(full_title: &str, verbose: bool) -> (Option<&str>, Option<&str>, Option<&str>) {
    let re = Regex::new(r"^「(?<genre>[^」]+)」\[(?<artist>[^\]]+)\]\s(?<title>.+)$").unwrap();

    let mut genre = None;
    let mut artist = None;
    let mut title = None;

    for caps in re.captures_iter(full_title) {
        if verbose {
            println!("Captures: {:?}", caps);
        }

        if let Some(m) = caps.name("genre") {
            genre = Some(m.as_str());
        }
        if let Some(m) = caps.name("artist") {
            artist = Some(m.as_str());
        }
        if let Some(m) = caps.name("title") {
            title = Some(m.as_str());
        }
    }

    (genre, artist, title)
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

    // The full title (used for tag extracting)
    let mut meta_title = String::from(meta_title);
    // The resulting actual track title (some info might be stripped / added)
    let mut title = meta_title.to_string();
    let mut author: Vec<String> = Vec::new();

    if let Some((authors, rest_title)) = from_split(&meta_title) {
        author.extend(authors);
        title = rest_title.clone();
        meta_title = rest_title;
    } else {
        let (genre, artist, rest_title) = from_format(&meta_title, verbose);
        if let Some(genre) = genre {
            tags.insert("genre", genre.to_string());
        }
        if let Some(artist) = artist {
            author.push(artist.to_string());
        }
        if let Some(rest_title) = rest_title {
            title = rest_title.to_string();
            meta_title = rest_title.to_string();
        }
    }

    let re = Regex::new(
        r"(?xi)
        (?<feat>\((\sand\s|featuring|feat\.?|ft\.?|w[⧸/])[^\)]*\)|(\sand\s|featuring|feat\.?|ft\.?|w[⧸/])[^\)]*) |
        (?<year>\(\d{4}\)|\d{4}) |
        (?<remix>[\[({<][^\[\](){}<>]*(edit|extended(\smix)?|(re)?mix|remaster|bootleg|instrumental)[^\[\](){}<>]*[\])}>]) |
        (?<album>【[^【】]*(?<album_rmv>F.C)[^【】]*】) |
        (?<strip>[\[({<][^\[\](){}<>]*((official\s)?(music\s)?video|m/?v|hq|hd)[^\[\](){}<>]*[\])}>])
        ",
    );

    for caps in re.unwrap().captures_iter(&meta_title) {
        if verbose {
            println!("Captures: {:?}", caps);
        }

        if let Some(feat) = caps.name("feat") {
            // Authors to the right of "-"
            let feat = feat.as_str();
            title = util::remove_str_from_string(title, feat);
            let feat = util::remove_brackets(feat);
            author.extend(separate_authors(&feat).into_iter().skip(1));
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

        if let Some(album) = caps.name("album") {
            let album = album.as_str();
            title = util::remove_str_from_string(title, album);

            let album = if let Some(album_rmv) = caps.name("album_rmv") {
                util::remove_str_from_string(album.to_string(), album_rmv.as_str())
            } else {
                String::from(album)
            };

            tags.insert("album", util::remove_brackets(&album));
        }

        if let Some(strip) = caps.name("strip") {
            title = util::remove_str_from_string(title, strip.as_str());
        }
    }

    tags.insert("title", title);

    if let Some(author) = author
        .iter()
        .map(|s| s.to_string())
        .reduce(|a, b| format!("{}&{}", a, b))
    {
        tags.insert("author", author);
    }

    if verbose {
        println!("Got tags: {:?}", tags);
    }

    Some(tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spacing() {
        let inputs = ["Band - Song", "Band- Song", "Band -Song", "Band-Song"];
        for song in inputs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], "Band");
            assert_eq!(tags["title"], "Song");
        }
    }

    #[test]
    fn parses_featuring_artists() {
        let inputs = [
            ("Artist & Band - Song", "Artist&Band"),
            ("Artist, Other & Another - Song", "Artist&Other&Another"),
            ("Artist ft. Other - Song", "Artist&Other"),
            ("Artist & Band feat. Other - Song", "Artist&Band&Other"),
            ("Soft Artist - Song", "Soft Artist"),
            ("Artist - Song (feat.Band)", "Artist&Band"),
            ("Artist - Song w/Band", "Artist&Band"),
            ("Artist - Song W/Band", "Artist&Band"),
        ];
        for (song, expected) in inputs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], expected);
        }
    }

    #[test]
    fn parses_year() {
        let year = String::from("2024");
        let inputs = [
            ("Band - Song (2024)", Some(&year)),
            ("Band - Song 2024", Some(&year)),
            ("Band - Song", None),
        ];
        for (song, expected) in inputs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], "Band");
            assert_eq!(tags["title"], "Song");
            assert_eq!(tags.get("year"), expected);
        }
    }

    #[test]
    fn parses_remix() {
        let inputs = [
            ("Band - Song [Club Remix]", "Club Remix"),
            ("Band - Song [Instrumental]", "Instrumental"),
            ("Band - Song (HQ REMASTER)", "HQ REMASTER"),
            ("Band - Song (Extended)", "Extended"),
            ("Band - Song (Extended Mix)", "Extended Mix"),
            ("Band - Song (Radio Edit)", "Radio Edit"),
            ("Band - Song (Edit)", "Edit"),
        ];
        for (song, expected) in inputs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], "Band");
            assert_eq!(tags["title"], "Song");
            assert_eq!(tags["remix"], expected);
        }
    }

    #[test]
    fn strips_useless_info() {
        let inputs = [
            ("Artist - Song [HQ]", "HQ"),
            ("Artist - Song [HD]", "HD"),
            ("Artist - Song [M/V]", "M/V"),
            (
                "Artist - Song (Official Music Video)",
                "Official Music Video",
            ),
            ("Artist - Song (Official Video)", "Official Video"),
            ("Artist - Song (Music Video)", "Music Video"),
        ];
        for (song, _expected) in inputs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], "Artist");
            assert_eq!(tags["title"], "Song");
            assert_eq!(tags.get("remix"), None);
            assert_eq!(tags.len(), 2);
            // assert_eq!(tags["strip"], expected);
        }
    }

    #[test]
    fn parses_complex_title() {
        let tags = build_tags("Artist & Band - Song (radio mix) 2003", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");
        assert_eq!(tags["title"], "Song");
        assert_eq!(tags["remix"], "radio mix");
        assert_eq!(tags["year"], "2003");
    }

    #[test]
    fn parses_from_format() {
        let tags = build_tags("「Deep House」[DJ Test] My House", true).unwrap();
        assert_eq!(tags["author"], "DJ Test");
        assert_eq!(tags["title"], "My House");
        assert_eq!(tags["genre"], "Deep House");
    }
}
