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
        println!("\nTagging {} of {}: {}", i + 1, total, filename);

        let ftag = Tag::new().read_from_path(entry);
        if let Err(e) = ftag {
            println!("! {}, skipping", e);
            continue;
        }
        let mut ftag = ftag.unwrap();

        let title = if let Some(title) = ftag.title() {
            String::from(title)
        } else {
            println!("! No 'title' tag present, skipping");
            continue;
        };

        let tags = if let Some(tags) = build_tags(title.trim(), config.verbose) {
            tags
        } else {
            println!("! No extra tags extracted from 'title', skipping");
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
    let re = Regex::new(
        r"(?i)(\sx\s|\sand\s|(^|\s)featuring|(^|\s)feat\.?|(^|\s)ft\.?|(^|\s)w[⧸/]|&|,|，)",
    );
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

/// Attempt to extract the following tags from the title:
/// - genre
/// - artist
/// - title
/// - extra
/// The 'extra' group can be used to capture anything extra for independent further extraction,
/// commonly this could be remix or featuring artist information.
///
/// # Parameters
/// - `format`: a regular expression with capture groups for any/all of the above tags
/// - `full_title`: expected to be in the provided format
///
/// # Returns
/// - `None`: if no tags were found (format could not capture anything)
/// - `Some(HashMap)`: map of tag name to tag value
fn from_format<'a, 'b>(
    format: &'a str,
    full_title: &'b str,
    verbose: bool,
) -> Option<HashMap<&'b str, &'b str>> {
    let mut tags = HashMap::new();

    for caps in Regex::new(format).unwrap().captures_iter(full_title) {
        if verbose {
            println!("Captures: {:?}", caps);
        }

        for name in ["genre", "artist", "title", "extra"] {
            if let Some(m) = caps.name(name) {
                tags.insert(name, m.as_str());
            }
        }
    }

    if tags.is_empty() {
        None
    } else {
        if verbose {
            println!("Got tags: {:?}", tags);
        }
        Some(tags)
    }
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
        // Attempt to parse the title with any of the formats, stop as soon as one format can parse it
        let formats = [
            r"^「(?<genre>[^」]+)」\[(?<artist>[^\]]+)\]\s(?<title>.+)$", // 「GENRE」[ARTIST] TITLE
            r"^(?<artist>[^'‘]+)\s['‘](?<title>[^'’]+)['’](?<extra>.+)?$", // ARTIST 'TITLE'EXTRA?
        ];
        for fmt in formats {
            if let Some(tt) = from_format(fmt, &meta_title, verbose) {
                if let Some(genre) = tt.get("genre") {
                    tags.insert("genre", genre.to_string());
                }
                if let Some(artist) = tt.get("artist") {
                    author.push(artist.to_string());
                }
                let rest_title = tt.get("title");
                let extra = tt.get("extra").unwrap_or(&"");
                if let Some(rest_title) = rest_title {
                    title = format!("{}{}", rest_title.to_string(), extra);
                    meta_title = format!("{}{}", rest_title.to_string(), extra);
                }
                break;
            }
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

    /// Holds the expected values that should be extracted from the input
    #[derive(Debug, Default)]
    struct Song {
        genre: Option<String>,
        author: Option<String>,
        title: Option<String>,
        remix: Option<String>,
        year: Option<String>,
    }
    impl Song {
        fn check(input: &str, song: Song) {
            let tags = build_tags(input, true).unwrap();
            assert_eq!(tags.get("genre"), song.genre.as_ref());
            assert_eq!(tags.get("author"), song.author.as_ref());
            assert_eq!(tags.get("title"), song.title.as_ref());
            assert_eq!(tags.get("remix"), song.remix.as_ref());
            assert_eq!(tags.get("year"), song.year.as_ref());
        }
    }
    macro_rules! song {
        ($author: expr, $title: expr) => {
            Song {
                author: Some(String::from($author)),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
        ($genre: expr, $author: expr, $title: expr) => {
            Song {
                genre: Some(String::from($genre)),
                author: Some(String::from($author)),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
    }
    macro_rules! year {
        ($author: expr, $title: expr, $year: expr) => {
            Song {
                author: Some(String::from($author)),
                title: Some(String::from($title)),
                year: Some(String::from($year)),
                ..Default::default()
            }
        };
    }
    macro_rules! rmx {
        ($author: expr, $title: expr, $remix: expr) => {
            Song {
                author: Some(String::from($author)),
                title: Some(String::from($title)),
                remix: Some(String::from($remix)),
                ..Default::default()
            }
        };
        ($author: expr, $title: expr, $remix: expr, $year: expr) => {
            Song {
                author: Some(String::from($author)),
                title: Some(String::from($title)),
                remix: Some(String::from($remix)),
                year: Some(String::from($year)),
                ..Default::default()
            }
        };
    }

    #[test]
    fn parses_spacing() {
        Song::check("Band - Song", song!("Band", "Song"));
        Song::check("Band- Song", song!("Band", "Song"));
        Song::check("Band -Song", song!("Band", "Song"));
        Song::check("Band-Song", song!("Band", "Song"));
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
        for (input_str, expected_output) in inputs {
            Song::check(input_str, song!(expected_output, "Song"));
        }
    }

    #[test]
    fn parses_year() {
        Song::check("Band - Song (2024)", year!("Band", "Song", "2024"));
        Song::check("Band - Song 2024", year!("Band", "Song", "2024"));
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
        for (input_str, expected_output) in inputs {
            Song::check(input_str, rmx!("Band", "Song", expected_output));
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
        for input_str in inputs {
            Song::check(input_str, song!("Artist", "Song"));
        }
    }

    #[test]
    fn parses_complex_formats() {
        Song::check("A & B - S (a mix) 2003", rmx!("A&B", "S", "a mix", "2003"));
        Song::check("「Big」[Band] Song", song!("Big", "Band", "Song"));
        Song::check("Artist 'Title'", song!("Artist", "Title"));
        Song::check("Artist 'Title' (Edit)", rmx!("Artist", "Title", "Edit"));
        Song::check("Artist ‘Title’ (Feat. Band)", song!("Artist&Band", "Title"));
    }
}
