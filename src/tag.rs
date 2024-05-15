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
        if let Some(a) = entry_tag.artist() {
            old_artist = Some(String::from(a));
            if !config.override_artist {
                artist = old_artist.clone();
            }
        }

        if let Some(author) = tags.get("author") {
            let mut artists = author.split("&");

            // First artist is seen as main
            if artist.is_none() {
                if let Some(a) = artists.next() {
                    artist = Some(String::from(a));
                }
            }

            if let Some(mut feat) = artists
                .map(|s| s.to_string())
                .reduce(|a, b| format!("{}, {}", a, b))
            {
                if let Some(i) = feat.rfind(',') {
                    feat.replace_range(i..=i, " &");
                }
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
        print_proposal("FILENAME", &Some(&filename), &Some(&new_filename));
        println!("Tags:");
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
        if new.is_none() {
            println!("{:<15} N/A", name);
        } else {
            println!("{:<15} N/A -> {}", name, new.as_ref().unwrap());
        }
    } else {
        let old = old.as_ref().unwrap();
        if new.is_none() || new.as_ref().is_some_and(|x| *x == *old) {
            println!("{:<15} {} -> unchanged", name, old);
        } else {
            println!("{:<15} {} -> {}", name, old, new.as_ref().unwrap());
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
    let mut author: Vec<String> = Vec::new();

    for delim in "-_~｜".chars() {
        if let Some((full_author, full_title)) = meta_title.split_once(delim) {
            // Authors to the left of "-", e.g. Band ft Artist, Musician & Singer - Song
            let author_re =
                Regex::new(r"(?i)(\sand\s|\sfeaturing|\sfeat\.?|\sft\.?|\sw[⧸/]|&|,)").unwrap();
            author.extend(author_re.split(full_author).map(|s| s.trim().to_string()));

            let full_title = full_title.trim();
            title = full_title.to_string();
            meta_title = full_title;
            break;
        }
    }

    let re = Regex::new(
        r"(?xi)
        (?<feat>\((\sand\s|featuring|feat\.?|ft\.?|w[⧸/])[^\)]*\)|(\sand\s|featuring|feat\.?|ft\.?|w[⧸/])[^\)]*) |
        (?<year>\(\d{4}\)|\d{4}) |
        (?<remix>[\[({<][^\[\](){}<>]*((re)?mix|remaster|bootleg|instrumental)[^\[\](){}<>]*[\])}>]) |
        (?<album>【[^【】]*(?<album_rmv>F.C)[^【】]*】) |
        (?<strip>[\[({<][^\[\](){}<>]*((official\s)?(music\s)?video|m/?v|hq|hd)[^\[\](){}<>]*[\])}>])
        ",
    );

    for caps in re.unwrap().captures_iter(meta_title) {
        if verbose {
            println!("Captures: {:?}", caps);
        }

        if let Some(feat) = caps.name("feat") {
            let feat = feat.as_str();
            title = util::remove_str_from_string(title, feat);
            let feat = util::remove_brackets(feat);
            // Authors to the right of "-", e.g. Band - Song (ft Artist, Musician & Singer)
            let author_re = Regex::new(r"(?i)(\sand\s|featuring|feat\.?|ft\.?|w[⧸/]|&|,)").unwrap();
            author.extend(author_re.split(&feat).skip(1).map(|s| s.trim().to_string()));
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

        let tags = build_tags("Soft Artist - Song", true).unwrap();
        assert_eq!(tags["author"], "Soft Artist");

        let tags = build_tags("Artist - Song (feat.Band)", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");

        let tags = build_tags("Artist - Song w/Band", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");

        let tags = build_tags("Artist - Song W/Band", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");

        let tags = build_tags("Artist And Band - Song", true).unwrap();
        assert_eq!(tags["author"], "Artist&Band");
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
