use crate::{types, util};
use std::collections::HashMap;
use std::io::{BufRead, Write};

/// # Returns
/// `HashMap<String, Option<String>>`:
/// - The `String` key is the tag name
/// - The `Option` is the value: `None` to clear it, `Some(String)` to set/update it
pub fn edit<R: BufRead>(mut reader: R) -> types::HashMapResult {
    println!("\n===== Tapeworm Tag Editor =====");
    tag_editor_help();

    let mut edits = HashMap::new();
    loop {
        print!("?> ");
        std::io::stdout().flush()?;
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
    let (tag_name, tag_value) = if let Some((k, v)) = cmd.split_once(' ') {
        (k.to_uppercase(), Some(String::from(v)))
    } else {
        (cmd.to_uppercase(), None)
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
Commands:
  quit, q         Go back to \"Proposed changes\" (asks to confirm your edits, if any)
  help, h         Show this help menu
  TAG             Clear TAG value
  TAG VALUE       Set TAG to VALUE (ARTIST may have multiple with ';'), e.g.: `ARTIST The Band;Singer`, `ARTIST Rapper`
Supported tags (lowercase also allowed):
  ARTIST, ALBUM, ALBUM_ARTIST, GENRE, TITLE, TRACK, YEAR");
}
