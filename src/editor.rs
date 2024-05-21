//! Tag editor functions.

use crate::{types, util};
use std::{collections::HashMap, io::BufRead};

/// # Returns
/// `HashMap<String, Option<String>>`:
/// - The `String` key is the tag name
/// - The `Option` is the value: `None` to clear it, `Some(String)` to set/update it
pub fn edit<R: BufRead>(mut reader: R) -> types::HashMapResult {
    println!("tte> ===== Tapeworm Tag Editor =====");
    tag_editor_help();

    let mut edits = HashMap::new();

    loop {
        println!("tte> ?: ");
        let cmd = util::input(&mut reader, false)?;
        match cmd.as_str() {
            "quit" | "q" => break,
            "help" | "h" => tag_editor_help(),
            _ => {
                if let Some((tag_name, tag_value)) = parse(cmd) {
                    edits.insert(tag_name, tag_value);
                } else {
                    println!("Unknown command, try 'help'");
                }
            }
        }
    }

    Ok(edits)
}

fn parse(cmd: String) -> Option<(String, Option<String>)> {
    let split = cmd.split_once(' ');
    let (tag_name, tag_value) = if split.is_none() {
        (cmd.to_uppercase(), None)
    } else {
        let (k, v) = split.unwrap();
        (k.to_uppercase(), Some(String::from(v)))
    };

    match tag_name.as_str() {
        "ARTIST" | "ALBUM" | "ALBUM_ARTIST" | "GENRE" | "TITLE" | "TRACK" | "YEAR" => {
            Some((tag_name, tag_value))
        }
        _ => None,
    }
}

fn tag_editor_help() {
    println!("\
tte> Commands:
tte>     quit, q         Go back to \"Proposed changes\" (asks to confirm your edits, if any)
tte>     help, h         Show this help menu
tte>     TAG             Clear TAG value
tte>     TAG VALUE       Set TAG to VALUE (ARTIST may have multiple with ';'), e.g.: `ARTIST The Band;Singer`, `ARTIST Rapper`
tte> Supported tags (lowercase also allowed):
tte>    ARTIST, ALBUM, ALBUM_ARTIST, GENRE, TITLE, TRACK, YEAR");
}
