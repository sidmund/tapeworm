//! This module provides functionality for extracting tags from a filename.

use crate::util::PromptOption::{Edit, No, Yes};
use crate::{editor, types, util, Config};
use audiotags::Tag;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::{fs, io::BufRead, path::PathBuf};

struct RegexConfig {
    author_separator: Regex,
    title_formats: Vec<Regex>,
    catch_all: Regex,
}
impl Default for RegexConfig {
    fn default() -> Self {
        Self {
            author_separator: Regex::new(
                r"(?i)(\s(x|and)\s|(^|\s)(feat(uring|\.)?|ft\.?|w[⧸/])|&|,|，)",
            )
            .unwrap(),
            title_formats: vec![
                // 「GENRE」[ARTISTS] TITLE
                Regex::new(r"^「(?<genre>[^」]+)」\[(?<artists>[^\]]+)\]\s(?<title>.+)$").unwrap(),
                // ARTISTS 'TITLE'EXTRA?
                Regex::new(r"^(?<artists>[^'‘]+)\s['‘](?<title>[^'’]+)['’](?<extra>.+)?$").unwrap(),
                // TRACK_NO.? ARTISTS - TITLE
                Regex::new(r"^(?<track_no>\d+\.)?(?<artists>[^-_~｜]+)[-_~｜](?<title>.+)$")
                    .unwrap(),
            ],
            catch_all: Regex::new(
        r"(?xi)
        (?<feat>\((\sand\s|feat(uring|\.)?|ft\.?|w[⧸/])[^\)]*\)|(\sand\s|feat(uring|\.)?|ft\.?|w[⧸/])[^\(\)]*) |
        (?<year>\(\d{4}\)|\d{4}) |
        (?<remix>[\[({<][^\[\](){}<>]*(cut|edit|extend(ed)?(\smix)?|(re)?mix|remaster|bootleg|instrumental)[^\[\](){}<>]*[\])}>]) |
        (?<album>[\[\(【][^\[\]\(\)【】]*(?<album_rmv>F.C)[^\[\]\(\)【】]*[\]\)】]) |
        (?<strip>[\[({<][^\[\](){}<>]*(full\sversion|(official\s)?((music\s)?video|audio)|m/?v|hq|hd)[^\[\](){}<>]*[\])}>])
        ",
            ).unwrap(),
        }
    }
}

/// For each downloaded file, use its "title" metadata tag to extract more tags. If this tag is not
/// present in the file, it will not be affected.
///
/// Titles generally contain extra information, e.g. "Artist ft. Band - Song (2024) [Remix]"
/// Information such as collaborating artists, year, remix, etc. are extracted.
pub fn tag<R: BufRead>(config: &Config, mut reader: R) -> types::UnitResult {
    if config.input_dir.is_none() {
        return Err("'INPUT_DIR' must be set. See 'help'".into());
    }

    let downloads =
        PathBuf::from(config.lib_path.clone().unwrap()).join(config.input_dir.clone().unwrap());
    let downloads: Vec<PathBuf> = util::filepaths_in(downloads)?;
    let total = downloads.len();

    let regex = RegexConfig::default();

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

        let tags = if let Some(tags) = build_tags(&regex, title.trim(), config.verbose) {
            tags
        } else {
            println!("! No extra tags extracted from 'title', skipping");
            continue;
        };

        let old_album = ftag.album().map(|a| String::from(a.title));
        let old_album_artist = ftag.album_artist().map(|a| String::from(a));
        let old_artist = ftag.artist().map(|a| String::from(a));

        let mut album = tags.get("album").map(|a| a.to_owned());
        let mut album_artist = None;
        let mut artist = None;
        let mut genre = tags.get("genre").map(|g| g.to_owned());
        let mut title = tags.get("title").map(|t| t.to_owned());
        let mut track_no = tags.get("track_no").map(|n| n.parse::<u16>().ok().unwrap());
        let mut year = tags.get("year").map(|y| y.parse::<i32>().ok().unwrap());

        // Obtain all artists
        let mut multiple = HashSet::new(); // No dupes
        if old_artist.is_some() && !config.override_artist {
            multiple.extend(separate_authors(&regex, &old_artist.clone().unwrap()));
        }
        if let Some(author) = tags.get("author") {
            multiple.extend(author.split("&").map(|s| s.to_string()));
        }
        let mut multiple: Vec<String> = multiple.into_iter().collect();

        loop {
            // Set the artist from the last found artist (order is random from HashSet)
            let mut featuring = String::new();
            for (i, a) in multiple.iter().enumerate() {
                if i == 0 {
                    artist = Some(String::from(a));
                } else if i == multiple.len() - 1 {
                    featuring.push_str(a);
                } else {
                    featuring.push_str(&format!("{}, ", String::from(a)));
                }
            }
            title = title_from(title, featuring, tags.get("remix"));
            let new_filename = filename_from(&filename, &artist, &old_artist, &title);

            // TODO move to a function
            println!("\nProposed changes:");
            print_proposal("ARTIST", &old_artist, &artist);
            if old_album.is_some() || album.is_some() {
                print_proposal("ALBUM_ARTIST", &old_album_artist, &album_artist);
            }
            print_proposal("ALBUM", &old_album, &album);
            print_proposal("TRACK", &ftag.track_number(), &track_no);
            print_proposal("TITLE", &ftag.title(), &title.as_ref().map(|s| s.as_str()));
            print_proposal("YEAR", &ftag.year(), &year);
            print_proposal("GENRE", &ftag.genre(), &genre.as_ref().map(|s| s.as_str()));
            print_proposal("FILENAME", &Some(&filename), &Some(&new_filename));

            let choice = util::confirm_with_options(
                "Accept these changes?",
                vec![Yes, No, Edit],
                Yes,
                &mut reader,
            )?;
            match choice {
                Edit => {
                    // TODO move to a function
                    let edits = editor::edit(&mut reader)?;
                    for (tag_name, tag_value) in edits {
                        match tag_name.as_str() {
                            "ARTIST" => {
                                if let Some(new_artist) = tag_value {
                                    let all: HashSet<String> =
                                        new_artist.split(";").map(|s| s.to_string()).collect();
                                    multiple = all.into_iter().collect();
                                } else {
                                    multiple.clear();
                                    artist = None;
                                }
                            }
                            "ALBUM" => album = tag_value,
                            "ALBUM_ARTIST" => album_artist = tag_value,
                            "GENRE" => genre = tag_value,
                            "TITLE" => title = tag_value,
                            "TRACK" => {
                                if let Some(new_track) = tag_value {
                                    if let Ok(new_track) = new_track.parse::<u16>() {
                                        track_no = Some(new_track);
                                    } else {
                                        println!("TRACK is not a valid number, ignoring");
                                    }
                                } else {
                                    track_no = None;
                                }
                            }
                            "YEAR" => {
                                if let Some(new_year) = tag_value {
                                    if let Ok(new_year) = new_year.parse::<i32>() {
                                        year = Some(new_year);
                                    } else {
                                        println!("YEAR is not a valid number, ignoring");
                                    }
                                } else {
                                    year = None;
                                }
                            }
                            _ => println!("Unsupported tag: '{}', skipping", tag_name),
                        }
                    }
                }
                Yes => {
                    // TODO move to a function
                    if let Some(artist) = artist.clone() {
                        ftag.set_artist(&artist);
                    }
                    if old_album.is_some() || album.is_some() {
                        if let Some(artist) = artist {
                            ftag.set_album_artist(&artist);
                        }
                    }
                    if let Some(album) = album {
                        ftag.set_album_title(&album);
                    }
                    if let Some(track_no) = track_no {
                        ftag.set_track_number(track_no);
                    }
                    if let Some(title) = title {
                        ftag.set_title(&title);
                    }
                    if let Some(year) = year {
                        ftag.set_year(year);
                    }
                    if let Some(genre) = genre {
                        ftag.set_genre(&genre);
                    }
                    ftag.write_to_path(entry.to_str().unwrap())?;
                    if new_filename != filename {
                        fs::rename(entry, entry.with_file_name(new_filename))?;
                    }
                    break;
                }
                _ => break, // No, don't write changes
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
            println!("  {:<15} N/A\n{:<16}> {}", name, "", new.as_ref().unwrap());
        } // No need to print anything when both are none
        return;
    }

    let old = old.as_ref().unwrap();
    if new.is_none() || new.as_ref().is_some_and(|x| *x == *old) {
        println!("  {:<15} (keep) {}", name, old);
    } else {
        let new = new.as_ref().unwrap();
        println!("  {:<15} {}\n{:<16}> {}", name, old, "", new);
    }
}

/// Modify `title` to incorporate featuring artists and remix information.
/// E.g.: "Theme song (Alice, Bob & Charlie) [Radio Edit]"
fn title_from(
    title: Option<String>,
    featuring_artists: String,
    remix: Option<&String>,
) -> Option<String> {
    let mut new_title = title.clone();
    if !featuring_artists.is_empty() {
        let mut feat = featuring_artists;
        if let Some(i) = feat.rfind(',') {
            feat.replace_range(i..=i, " &");
        }
        new_title = Some(format!("{} ({})", title.unwrap_or(String::new()), feat));
    }
    if let Some(remix) = remix {
        new_title = Some(format!(
            "{} [{}]",
            new_title.unwrap_or(String::new()),
            remix
        ));
    }
    new_title
}

fn filename_from(
    filename: &String,
    artist: &Option<String>,
    old_artist: &Option<String>,
    title: &Option<String>,
) -> String {
    if let Some(artist) = artist {
        if let Some(title) = title {
            format!("{} - {}.mp3", artist, title)
        } else {
            format!("{}.mp3", artist)
        }
    } else if let Some(title) = title {
        if let Some(tag_artist) = old_artist {
            // When filename led to only title being extracted, but the artist tag was set by
            // yt-dlp, e.g. "Song.mp3" only gives tags "title: Song" but yt-dlp set the artist
            format!("{} - {}.mp3", tag_artist, title)
        } else {
            format!("{}.mp3", title)
        }
    } else {
        String::from(filename)
    }
}

/// Separates a string like "Band ft Artist, Musician & Singer"
/// into a vector like ["Band", "Artist", "Musician", "Singer"].
fn separate_authors(regex: &RegexConfig, s: &str) -> Vec<String> {
    regex
        .author_separator
        .split(s)
        .map(|a| a.trim().to_string())
        .collect()
}

/// Attempt to extract the following tags from the title:
/// - genre
/// - artists: can be a single artist or multiple, e.g. "Band", "Artist ft Singer"
/// - title
/// - track_no
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
fn from_format<'a>(
    format: &Regex,
    full_title: &'a str,
    verbose: bool,
) -> Option<HashMap<&'a str, &'a str>> {
    let mut tags = HashMap::new();

    for caps in format.captures_iter(full_title) {
        if verbose {
            println!("\nUsing Regex: {}\n{:#?}", format, caps);
        }

        for name in ["genre", "artists", "title", "track_no", "extra"] {
            if let Some(m) = caps.name(name) {
                tags.insert(name, m.as_str());
            }
        }
    }

    if tags.is_empty() {
        None
    } else {
        if verbose {
            println!("Got tags:\n{:#?}", tags);
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
/// - track_no
///
/// # Returns
/// - `None`: when the title is empty, or no tags were found
/// - `Some(HashMap<&str, String>)`: the map of found tags, contains at least the 'title'
fn build_tags<'a>(
    regex: &RegexConfig,
    meta_title: &'a str,
    verbose: bool,
) -> Option<HashMap<&'a str, String>> {
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

    for fmt in &regex.title_formats {
        if let Some(tt) = from_format(fmt, &meta_title, verbose) {
            if let Some(genre) = tt.get("genre") {
                tags.insert("genre", genre.to_string());
            }
            if let Some(track_no) = tt.get("track_no") {
                let track_no = track_no.to_string();
                title = util::remove_str_from_string(title, &track_no);
                let track_no = String::from(&track_no[..track_no.len() - 1]); // Omit "."
                tags.insert("track_no", track_no);
            }
            if let Some(artists) = tt.get("artists") {
                author.extend(separate_authors(regex, artists));
            }
            let rest_title = tt.get("title");
            let extra = tt.get("extra").unwrap_or(&"");
            if let Some(rest_title) = rest_title {
                let rest_title = rest_title.trim();
                let extra = extra.trim();
                title = format!("{}{}", rest_title.to_string(), extra);
                meta_title = format!("{}{}", rest_title.to_string(), extra);
            }
            break; // Stop as soon as one format can parse it
        }
    }

    for caps in regex.catch_all.captures_iter(&meta_title) {
        if verbose {
            println!("{:#?}", caps);
        }

        if let Some(feat) = caps.name("feat") {
            // Authors to the right of "-"
            let feat = feat.as_str();
            title = util::remove_str_from_string(title, feat);
            let feat = util::remove_brackets(feat);
            author.extend(separate_authors(regex, &feat).into_iter().skip(1));
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
        println!("Got tags:\n{:#?}", tags);
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
        track_no: Option<String>,
    }
    impl Song {
        fn check(regex: &RegexConfig, input: &str, song: Song) {
            let tags = build_tags(regex, input, true).unwrap();
            assert_eq!(tags.get("genre"), song.genre.as_ref());
            assert_eq!(tags.get("author"), song.author.as_ref());
            assert_eq!(tags.get("title"), song.title.as_ref());
            assert_eq!(tags.get("remix"), song.remix.as_ref());
            assert_eq!(tags.get("year"), song.year.as_ref());
            assert_eq!(tags.get("track_no"), song.track_no.as_ref());
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
    macro_rules! album {
        ($track_no: expr, $author: expr, $title: expr) => {
            Song {
                track_no: Some(String::from($track_no)),
                author: Some(String::from($author)),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
    }

    #[test]
    fn parses_spacing() {
        let r = RegexConfig::default();
        Song::check(&r, "Band - Song", song!("Band", "Song"));
        Song::check(&r, "Band- Song", song!("Band", "Song"));
        Song::check(&r, "Band -Song", song!("Band", "Song"));
        Song::check(&r, "Band-Song", song!("Band", "Song"));
    }

    #[test]
    fn parses_featuring_artists() {
        let r = RegexConfig::default();
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
            Song::check(&r, input_str, song!(expected_output, "Song"));
        }
    }

    #[test]
    fn parses_year() {
        let r = RegexConfig::default();
        Song::check(&r, "Band - Song (2024)", year!("Band", "Song", "2024"));
        Song::check(&r, "Band - Song 2024", year!("Band", "Song", "2024"));
    }

    #[test]
    fn parses_track_number() {
        let r = RegexConfig::default();
        Song::check(&r, "04. Band - Song", album!("04", "Band", "Song"));
    }

    #[test]
    fn parses_remix() {
        let r = RegexConfig::default();
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
            Song::check(&r, input_str, rmx!("Band", "Song", expected_output));
        }
    }

    #[test]
    fn strips_useless_info() {
        let r = RegexConfig::default();
        let inputs = [
            "Artist - Song [HQ]",
            "Artist - Song [HD]",
            "Artist - Song [M/V]",
            "Artist - Song (Official Music Video)",
            "Artist - Song (Official Video)",
            "Artist - Song (Official HD Video)",
            "Artist - Song (Official Audio)",
            "Artist - Song (Music Video)",
            "Artist - Song [Original Mix]",
            "Artist - Song [Full version]",
        ];
        for input_str in inputs {
            Song::check(&r, input_str, song!("Artist", "Song"));
        }
    }

    #[test]
    fn parses_complex_formats() {
        let r = RegexConfig::default();
        Song::check(
            &r,
            "A & B - S (a mix) 2003",
            rmx!("A&B", "S", "a mix", "2003"),
        );
        Song::check(&r, "「Big」[Band] Song", song!("Big", "Band", "Song"));
        Song::check(&r, "Artist 'Title'", song!("Artist", "Title"));
        Song::check(&r, "Artist 'Title' (Edit)", rmx!("Artist", "Title", "Edit"));
        Song::check(
            &r,
            "Artist ‘Title’ (Feat. Band)",
            song!("Artist&Band", "Title"),
        );
    }
}
