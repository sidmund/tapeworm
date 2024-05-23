//! This module provides functionality for extracting tags from a filename.

use crate::util::PromptOption::{Edit, No, Yes};
use crate::{editor, types, util, Config};
use audiotags::{AudioTag, Tag};
use regex::Regex;
use std::collections::HashMap;
use std::{fs, io::BufRead, path::PathBuf};

struct RegexConfig {
    artist_separator: Regex,
    title_formats: Vec<Regex>,
    catch_all: Regex,
}
impl Default for RegexConfig {
    fn default() -> Self {
        Self {
            artist_separator: Regex::new(
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
        (?<remix>[\[(][^\[\]()]*(cut|edit|extend(ed)?(\smix)?|(re)?mix|remaster|bootleg|instrumental)[^\[\]()]*[\])]) |
        (?<album>[\[\(【][^\[\]\(\)【】]*(?<album_rmv>F\WC)[^\[\]\(\)【】]*[\]\)】]) |
        (?<strip>[\[(][^\[\]()]*(full\sversion|(official\s)?((music\s)?video|audio)|m/?v|hq|hd)[^\[\]()]*[\])])
        ",
            ).unwrap(),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
struct TagProposal {
    album: Option<String>,
    album_artist: Option<String>,
    all_artists: Option<Vec<String>>,
    artist: Option<String>,
    filename: String,
    final_title: Option<String>,
    genre: Option<String>,
    remix: Option<String>,
    title: Option<String>,
    track: Option<u16>,
    year: Option<i32>,
}
impl TagProposal {
    fn feature(&mut self, artists: Vec<String>) {
        if self.all_artists.is_none() {
            self.all_artists = Some(Vec::with_capacity(artists.len()));
        }

        for artist in artists {
            if !self.all_artists.as_ref().unwrap().contains(&artist) {
                self.all_artists.as_mut().unwrap().push(artist);
            }
        }
    }

    /// Update the `artist` field based on the first artist of the `all_artists` field,
    /// and update the (original) `title` and `filename` based on provided templates.
    fn update(&mut self, title_template: &String, filename_template: &String) {
        let mut feat = String::new();
        if let Some(featuring) = &self.all_artists {
            for (i, a) in featuring.iter().enumerate() {
                if i == 0 {
                    self.artist = Some(String::from(a));
                } else if i == featuring.len() - 1 {
                    feat.push_str(a);
                } else {
                    feat.push_str(&format!("{}, ", String::from(a)));
                }
            }
            if !feat.is_empty() {
                if let Some(i) = feat.rfind(',') {
                    feat.replace_range(i..=i, " &");
                }
            }
        }

        self.final_title = Some(self.apply_template(&feat, &self.title, title_template));
        self.filename = self.apply_template(&feat, &self.final_title, filename_template);
    }

    fn present(&self, ftag: &Box<dyn AudioTag + Sync + Send>, old_filename: &String) {
        let album = self.album.as_ref().map(|s| s.as_str());
        let album_artist = self.album_artist.as_ref().map(|s| s.as_str());
        let artist = self.artist.as_ref().map(|s| s.as_str());
        let genre = self.genre.as_ref().map(|s| s.as_str());
        let title = self.final_title.as_ref().map(|s| s.as_str());

        println!("\nProposed changes:");
        print_proposal("ARTIST", &ftag.artist(), &artist);
        print_proposal("ALBUM_ARTIST", &ftag.album_artist(), &album_artist);
        print_proposal("ALBUM", &ftag.album_title(), &album);
        print_proposal("TRACK", &ftag.track_number(), &self.track);
        print_proposal("TITLE", &ftag.title(), &title);
        print_proposal("YEAR", &ftag.year(), &self.year);
        print_proposal("GENRE", &ftag.genre(), &genre);
        print_proposal("FILENAME", &Some(old_filename), &Some(&self.filename));
    }

    fn edit<R: BufRead>(&mut self, mut reader: R) -> types::UnitResult {
        for (tag_name, tag_value) in editor::edit(&mut reader)? {
            match tag_name.as_str() {
                "ARTIST" => {
                    self.all_artists = None;
                    if let Some(artists) = tag_value {
                        self.feature(artists.split(";").map(|s| s.to_string()).collect());
                    }
                }
                "ALBUM" => self.album = tag_value,
                "ALBUM_ARTIST" => self.album_artist = tag_value,
                "GENRE" => self.genre = tag_value,
                "TITLE" => self.title = tag_value,
                "TRACK" => {
                    if let Ok(track) = util::parse::<u16>(tag_value) {
                        self.track = track;
                    } else {
                        println!("TRACK is not a valid number, ignoring");
                    }
                }
                "YEAR" => {
                    if let Ok(year) = util::parse::<i32>(tag_value) {
                        self.year = year;
                    } else {
                        println!("YEAR is not a valid number, ignoring");
                    }
                }
                _ => println!("Unsupported tag: '{}', skipping", tag_name),
            }
        }

        Ok(())
    }

    fn accept(
        self,
        mut ftag: Box<dyn AudioTag + Sync + Send>,
        entry: &PathBuf,
    ) -> types::UnitResult {
        if let Some(s) = self.album {
            ftag.set_album_title(&s);
        }
        if let Some(s) = self.album_artist {
            ftag.set_album_artist(&s);
        }
        if let Some(s) = self.genre {
            ftag.set_genre(&s);
        }
        if let Some(s) = self.artist {
            ftag.set_artist(&s);
        }
        if let Some(s) = self.final_title {
            ftag.set_title(&s);
        }
        if let Some(i) = self.track {
            ftag.set_track_number(i);
        }
        if let Some(i) = self.year {
            ftag.set_year(i);
        }
        ftag.write_to_path(entry.to_str().unwrap())?;

        let mut to = entry.with_file_name(self.filename);
        if let Some(ext) = entry.extension() {
            to.set_extension(ext);
        }
        if to != entry.file_name().unwrap() {
            fs::rename(entry, to)?;
        }

        Ok(())
    }

    fn apply_template(&self, feat: &String, title: &Option<String>, template: &String) -> String {
        let mut s = template.clone();

        s = s.replace("{album}", self.album.as_ref().unwrap_or(&String::new()));
        s = s.replace(
            "{album_artist}",
            self.album_artist.as_ref().unwrap_or(&String::new()),
        );
        s = s.replace("{artist}", self.artist.as_ref().unwrap_or(&String::new()));
        s = s.replace("{feat}", feat);
        s = s.replace("{genre}", self.genre.as_ref().unwrap_or(&String::new()));
        s = s.replace("{remix}", self.remix.as_ref().unwrap_or(&String::new()));
        s = s.replace("{title}", title.as_ref().unwrap_or(&String::new()));
        if let Some(track) = &self.track {
            s = s.replace("{track}", &format!("{}", track));
        } else {
            s = s.replace("{track}", "");
        }
        if let Some(year) = &self.year {
            s = s.replace("{year}", &format!("{}", year));
        } else {
            s = s.replace("{year}", "");
        }

        String::from(util::remove_duplicate_whitespace(util::remove_empty_brackets(s)).trim())
    }
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

/// For each downloaded file, use its "title" metadata tag to extract more tags. If this tag is not
/// present in the file, it will not be affected.
///
/// Titles generally contain extra information, e.g. "Artist ft. Band - Song (2024) [Remix]"
/// Information such as collaborating artists, year, remix, etc. are extracted.
pub fn run<R: BufRead>(config: &Config, mut reader: R) -> types::UnitResult {
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
        let ftag = ftag.unwrap();

        let title = if let Some(title) = ftag.title() {
            String::from(title)
        } else {
            println!("! No 'title' tag present, skipping");
            continue;
        };

        let mut proposal = if let Some(p) = build_tags(&regex, title.trim(), config.verbose) {
            p
        } else {
            println!("! No extra tags extracted from 'title', skipping");
            continue;
        };

        let old_artist = ftag.artist().map(|a| String::from(a));
        if old_artist.is_some() && !config.override_artist {
            proposal.feature(separate_artists(&regex, &old_artist.clone().unwrap()));
        }

        loop {
            proposal.update(&config.title_template, &config.filename_template);
            proposal.present(&ftag, &filename);
            let choice =
                util::confirm_with_options("Accept?", vec![Yes, No, Edit], Yes, &mut reader)?;
            match choice {
                Edit => proposal.edit(&mut reader)?,
                Yes => {
                    proposal.accept(ftag, entry)?;
                    break;
                }
                _ => break, // No, don't write changes
            }
        }
    }

    Ok(())
}

/// Separates a string like "Band ft Artist, Musician & Singer"
/// into a vector like ["Band", "Artist", "Musician", "Singer"].
fn separate_artists(regex: &RegexConfig, s: &str) -> Vec<String> {
    regex
        .artist_separator
        .split(s)
        .filter(|a| !a.is_empty())
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

/// Extract tags from the title metadata.
///
/// # Returns
/// - `None`: when the title is empty
/// - `TagProposal`: the found tags, contains at least the sanitized 'title'
fn build_tags<'a>(regex: &RegexConfig, meta_title: &str, verbose: bool) -> Option<TagProposal> {
    if meta_title.is_empty() {
        return None;
    }

    if verbose {
        println!("Parsing: {}", meta_title);
    }

    let mut proposal = TagProposal::default();

    // The full title (used for tag extracting)
    let mut meta_title = String::from(meta_title);
    // The resulting actual track title (some info might be stripped / added)
    let mut title = meta_title.to_string();

    for fmt in &regex.title_formats {
        if let Some(tt) = from_format(fmt, &meta_title, verbose) {
            if let Some(genre) = tt.get("genre") {
                proposal.genre = Some(genre.to_string());
            }
            if let Some(track_no) = tt.get("track_no") {
                let track_no = track_no.to_string();
                title = util::remove_str_from_string(title, &track_no);
                let track_no = String::from(&track_no[..track_no.len() - 1]); // Omit "."
                proposal.track = track_no.parse::<u16>().ok();
            }
            if let Some(artists) = tt.get("artists") {
                proposal.feature(separate_artists(regex, artists));
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
            proposal.feature(separate_artists(regex, &feat));
        }

        if let Some(year) = caps.name("year") {
            let year = year.as_str();
            title = util::remove_str_from_string(title, year);
            proposal.year = util::remove_brackets(year).parse::<i32>().ok();
        }

        if let Some(remix) = caps.name("remix") {
            let remix = remix.as_str();
            title = util::remove_str_from_string(title, remix);
            let remix = util::remove_brackets(remix);
            if remix.to_lowercase() != "original mix" {
                proposal.remix = Some(remix);
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

            proposal.album = Some(util::remove_brackets(&album));
        }

        if let Some(strip) = caps.name("strip") {
            title = util::remove_str_from_string(title, strip.as_str());
        }
    }

    proposal.title = Some(title);

    if verbose {
        println!("Got tags:\n{:?}", proposal);
    }
    Some(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(regex: &RegexConfig, input: &str, expected: TagProposal) {
        assert_eq!(build_tags(regex, input, true).unwrap(), expected);
    }

    macro_rules! song {
        ($artists: expr, $title: expr) => {
            TagProposal {
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
        ($genre: expr, $artists: expr, $title: expr) => {
            TagProposal {
                genre: Some(String::from($genre)),
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
    }
    macro_rules! year {
        ($artists: expr, $title: expr, $year: expr) => {
            TagProposal {
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                year: Some($year),
                ..Default::default()
            }
        };
    }
    macro_rules! rmx {
        ($artists: expr, $title: expr, $remix: expr) => {
            TagProposal {
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                remix: Some(String::from($remix)),
                ..Default::default()
            }
        };
        ($artists: expr, $title: expr, $remix: expr, $year: expr) => {
            TagProposal {
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                remix: Some(String::from($remix)),
                year: Some($year),
                ..Default::default()
            }
        };
    }
    macro_rules! track {
        ($track_no: expr, $artists: expr, $title: expr) => {
            TagProposal {
                track: Some($track_no),
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
    }
    macro_rules! album {
        ($album: expr, $artists: expr, $title: expr) => {
            TagProposal {
                album: Some(String::from($album)),
                all_artists: Some($artists.split(';').map(String::from).collect()),
                title: Some(String::from($title)),
                ..Default::default()
            }
        };
    }

    #[test]
    fn parses_spacing() {
        let r = RegexConfig::default();
        check(&r, "Band - Song", song!("Band", "Song"));
        check(&r, "Band- Song", song!("Band", "Song"));
        check(&r, "Band -Song", song!("Band", "Song"));
        check(&r, "Band-Song", song!("Band", "Song"));
    }

    #[test]
    fn parses_featuring_artists() {
        let r = RegexConfig::default();
        let inputs = [
            ("Artist & Band - Song", "Artist;Band"),
            ("Artist, Other & Another - Song", "Artist;Other;Another"),
            ("Artist ft. Other - Song", "Artist;Other"),
            ("Artist & Band feat. Other - Song", "Artist;Band;Other"),
            ("Soft Artist - Song", "Soft Artist"),
            ("Artist - Song (feat.Band)", "Artist;Band"),
            ("Artist - Song w/Band", "Artist;Band"),
            ("Artist - Song W/Band", "Artist;Band"),
            ("Artist ， Band - Song", "Artist;Band"),
            ("Artist x Band - Song", "Artist;Band"),
        ];
        for (input_str, expected_output) in inputs {
            check(&r, input_str, song!(expected_output, "Song"));
        }
    }

    #[test]
    fn parses_year() {
        let r = RegexConfig::default();
        check(&r, "Band - Song (2024)", year!("Band", "Song", 2024));
        check(&r, "Band - Song 2024", year!("Band", "Song", 2024));
    }

    #[test]
    fn parses_track_number() {
        let r = RegexConfig::default();
        check(&r, "04. Band - Song", track!(4, "Band", "Song"));
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
            check(&r, input_str, rmx!("Band", "Song", expected_output));
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
            check(&r, input_str, song!("Artist", "Song"));
        }
    }

    #[test]
    fn parses_complex_formats() {
        let r = RegexConfig::default();
        check(&r, "A & B - S (mix) 2003", rmx!("A;B", "S", "mix", 2003));
        check(&r, "「Big」[Band] Song", song!("Big", "Band", "Song"));
        check(&r, "Artist 'Title'", song!("Artist", "Title"));
        check(&r, "Artist 'Title' (Edit)", rmx!("Artist", "Title", "Edit"));
        check(&r, "A ‘Title’ (Feat. B)", song!("A;B", "Title"));
        check(&r, "A - Title (F/C Vibes)", album!("Vibes", "A", "Title"));
    }

    #[test]
    fn generates_filename_from_template() {
        let title_template = String::from("{title} ({feat}) [{remix}]");
        let filename_template = String::from("{artist} - {title}");

        let inputs = [
            (TagProposal::default(), "-"),
            (song!("Artist", "Song"), "Artist - Song"),
            (song!("A;B;C", "Song"), "A - Song (B & C)"),
            (rmx!("Artist", "Song", "Remix"), "Artist - Song [Remix]"),
            (rmx!("A;B", "Song", "Edit"), "A - Song (B) [Edit]"),
        ];
        for (mut proposal, expected) in inputs {
            proposal.update(&title_template, &filename_template);
            assert_eq!(proposal.filename, expected);
        }
    }
}
