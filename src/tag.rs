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
    let downloads: Vec<PathBuf> = util::filepaths_in(downloads)?;
    let total = downloads.len();

    for (i, entry) in downloads.iter().enumerate() {
        let filename = entry.file_name().unwrap().to_owned().into_string().unwrap();
        println!("Tagging {} of {}: {}", i + 1, total, filename);

        let mut ftag = Tag::new().read_from_path(entry)?;

        let title = ftag.title().unwrap_or("").to_string();
        let tags = if let Some(tags) = build_tags(title.as_str(), config.verbose) {
            tags
        } else {
            println!("  No 'title' tag, or no extra info extracted, skipping");
            continue;
        };

        let year = tags.get("year").map(|y| y.parse::<i32>().ok().unwrap());
        let genre = tags.get("genre").map(|g| g.as_str());
        let old_album = ftag.album().map(|a| String::from(a.title));
        let album = tags.get("album").map(|a| a.to_owned());
        let old_album_artist = ftag.album_artist().map(|a| String::from(a));
        let old_artist = ftag.artist().map(|a| String::from(a));
        let mut artist: Option<String> = None;
        let mut title = tags.get("title").map(|t| t.to_owned());

        // Obtain all artists
        let mut multiple = HashSet::new(); // No dupes
        if old_artist.is_some() && !config.override_artist {
            multiple.extend(separate_authors(&old_artist.clone().unwrap()));
        }
        if let Some(author) = tags.get("author") {
            multiple.extend(author.split("&").map(|s| s.to_string()));
        }

        if !multiple.is_empty() {
            let mut multiple = multiple.iter().map(|s| s.to_owned());
            artist = Some(multiple.next().unwrap()); // First is treated as main artist

            // Modify the title with 'featuring' info for the remaining artists, e.g. "Song (Alice, Bob & Charlie)"
            if let Some(mut feat) = multiple.reduce(|a, b| format!("{}, {}", a, b)) {
                if let Some(i) = feat.rfind(',') {
                    feat.replace_range(i..=i, " &");
                }
                title = Some(format!("{} ({})", title.unwrap_or(String::new()), feat));
            }
        }

        // Modify the title with 'remix' info, e.g. "Song (Alice) [Radio Edit]"
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
            if let Some(tag_artist) = ftag.artist() {
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
        print_proposal("TITLE", &ftag.title(), &title.as_ref().map(|s| s.as_str()));
        print_proposal("YEAR", &ftag.year(), &year);
        print_proposal("GENRE", &ftag.genre(), &genre);
        print_proposal("FILENAME", &Some(&filename), &Some(&new_filename));

        if util::confirm("Accept these changes?", true)? {
            // Write tags
            if let Some(artist) = artist.clone() {
                ftag.set_artist(&artist);
            }
            if old_album.is_some() || album.is_some() {
                if let Some(artist) = artist {
                    ftag.set_album_artist(&artist);
                }
            }
            if let Some(album) = album {
                ftag.set_album_title(album.as_str());
            }
            if let Some(title) = title {
                ftag.set_title(title.as_str());
            }
            if let Some(year) = year {
                ftag.set_year(year);
            }
            if let Some(genre) = genre {
                ftag.set_genre(genre);
            }
            ftag.write_to_path(entry.to_str().unwrap())?;

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
        return;
    }

    let old = old.as_ref().unwrap();
    if new.is_none() || new.as_ref().is_some_and(|x| *x == *old) {
        println!("  {:<15} {}\n{:<20}(keep as is)", name, old, "");
    } else {
        let new = new.as_ref().unwrap();
        println!("  {:<15} {}\n{:<20}-> {}", name, old, "", new);
    }
}

/// Separates a string like "Band ft Artist, Musician & Singer"
/// into a vector like ["Band", "Artist", "Musician", "Singer"].
fn separate_authors(s: &str) -> Vec<String> {
    let re = Regex::new(r"(?i)(\sx\s|\sand\s|(^|\s)featuring|(^|\s)feat\.?|(^|\s)ft\.?|(^|\s)w[⧸/]|&|,|，)");
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
    let re = Regex::new(r"^「(?<genre>[^」]+)」\[(?<artist>[^\]]+)\]\s(?<title>.+)$");

    let mut genre = None;
    let mut artist = None;
    let mut title = None;

    for caps in re.unwrap().captures_iter(full_title) {
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

/// Extract tags from the title metadata. It will attempt to extract the following:
///
/// - author: a '&' separated list of authors
/// - title: the title after removing other/spurious information
/// - year
/// - genre
/// - remix: (re)mixes, remasters, bootlegs, instrumental are treated as 'remix'
///
/// # Returns
/// - `None`: when the title is empty, or no tags were found
/// - `Some(HashMap<&str, String>)`: the map of found tags, contains at least the 'title'
fn build_tags(meta_title: &str, verbose: bool) -> Option<HashMap<&str, String>> {
    if meta_title.is_empty() {
        return None;
    }

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
        (?<remix>[\[({<][^\[\](){}<>]*(cut|edit|extended(\smix)?|(re)?mix|remaster|bootleg|instrumental)[^\[\](){}<>]*[\])}>]) |
        (?<album>【[^【】]*(?<album_rmv>F.C)[^【】]*】) |
        (?<strip>[\[({<][^\[\](){}<>]*(full\sversion|(official\s)?(music\s)?video|m/?v|hq|hd)[^\[\](){}<>]*[\])}>])
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
            let remix = util::remove_brackets(remix);
            if remix.to_lowercase() != "original mix" {
                tags.insert("remix", remix);
            }
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
            ("Artist ， Band - Song", "Artist&Band"),
            ("Artist x Band - Song", "Artist&Band"),
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
            ("Band - Song (Radio Cut)", "Radio Cut"),
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
            "Artist - Song [HQ]",
            "Artist - Song [HD]",
            "Artist - Song [M/V]",
            "Artist - Song (Official Music Video)",
            "Artist - Song (Official Video)",
            "Artist - Song (Music Video)",
            "Artist - Song [Original Mix]",
            "Artist - Song [Full version]",
        ];
        for song in inputs {
            let tags = build_tags(song, true).unwrap();
            assert_eq!(tags["author"], "Artist");
            assert_eq!(tags["title"], "Song");
            assert_eq!(tags.get("remix"), None);
            assert_eq!(tags.len(), 2);
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
