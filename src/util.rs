use crate::types;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(PartialEq)]
pub enum PromptOption {
    Edit,
    No,
    Yes,
    YesToAll,
}

impl std::fmt::Display for PromptOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptOption::Edit => write!(f, "e"),
            PromptOption::No => write!(f, "n"),
            PromptOption::Yes => write!(f, "y"),
            PromptOption::YesToAll => write!(f, "a"),
        }
    }
}

impl PromptOption {
    fn info(&self) -> String {
        match self {
            PromptOption::Edit => String::from("Edit"),
            PromptOption::No => String::from("No"),
            PromptOption::Yes => String::from("Yes"),
            PromptOption::YesToAll => String::from("yes to All"),
        }
    }
}

/// Read a line of user input.
///
/// # Parameters
/// - `lowercase`: whether to convert the input to lowercase
///
/// # Returns
/// `String` with leading and trailing whitespace removed
pub fn input<R: BufRead>(mut reader: R, lowercase: bool) -> types::StringResult {
    let mut input = String::new();
    reader.read_line(&mut input)?;
    if lowercase {
        Ok(input.trim().to_lowercase())
    } else {
        Ok(input.trim().to_string())
    }
}

/// Prompt the user to select an option.
///
/// # Returns
/// `PromptOption`: the selected option, `default` if the user pressed 'Enter'
pub fn select<R: BufRead>(
    prompt: &str,
    options: Vec<PromptOption>,
    default: PromptOption,
    mut reader: R,
) -> types::PromptOptionResult {
    if options.is_empty() {
        return Err("Must specify at least one option".into());
    }

    let mut question = format!("{} ", prompt);
    let mut info = String::new();
    for option in &options {
        if default == *option {
            question = question + &format!("{}/", option).to_uppercase();
        } else {
            question = question + &format!("{}/", option);
        }
        info = format!("{}{}, ", info, option.info());
    }
    question.pop(); // Remove trailing '/'
    info.pop(); // Remove trailing ' '
    info.pop(); // Remove trailing ','
    print!("{} ({}) ", question, info);
    std::io::stdout().flush()?;

    let input = input(&mut reader, true)?;
    match input.chars().nth(0) {
        Some('e') if options.contains(&PromptOption::Edit) => Ok(PromptOption::Edit),
        Some('n') if options.contains(&PromptOption::No) => Ok(PromptOption::No),
        Some('y') if options.contains(&PromptOption::Yes) => Ok(PromptOption::Yes),
        Some('a') if options.contains(&PromptOption::YesToAll) => Ok(PromptOption::YesToAll),
        Some(_) => {
            println!("Invalid option. Please try again");
            select(prompt, options, default, reader)
        }
        None => Ok(default),
    }
}

/// Append the `content` to the file at `path`
pub fn append<P: AsRef<Path>>(path: P, content: String) -> types::UnitResult {
    Ok(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(content.as_bytes())?)
}

/// Overwrite the `content` to the file at `path`
pub fn write<P: AsRef<Path>>(path: P, content: String) -> types::UnitResult {
    Ok(fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?
        .write_all(content.as_bytes())?)
}

/// Create the directory if it does not exist.
///
/// # Parameters
/// - `dir`: the path to the directory
///
/// # Returns
/// `dir`, guaranteed to exist
pub fn guarantee_dir_path(dir: PathBuf) -> types::PathBufResult {
    if fs::metadata(&dir).is_err() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// # Returns
/// - `Err`: if the `dir` path does not exist
/// - `Vec<PathBuf>`: a list of files present, may be empty
pub fn filepaths_in(dir: &PathBuf) -> types::VecPathBufResult {
    Ok(fs::read_dir(dir)?
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|t| t.file_type().is_ok_and(|f| f.is_file()))
        })
        .map(|e| e.unwrap().path())
        .collect())
}

/// Parse a `Option<String>` into an `Option<F>`.
///
/// # Returns
/// - `Err` if parsing failed
/// - `Option<F>` on success
pub fn parse<F: FromStr>(value: Option<String>) -> Result<Option<F>, Box<dyn std::error::Error>> {
    if let Some(value) = value {
        if let Ok(value) = value.parse::<F>() {
            Ok(Some(value))
        } else {
            Err(format!("Invalid value: {}", value).into())
        }
    } else {
        Ok(None)
    }
}

/// Remove a string in its entirety from another string.
pub fn remove_str_from_string(s: String, to_remove: &str) -> String {
    String::from(s.split(to_remove).fold(String::new(), |a, s| a + s).trim())
}

/// Remove leading and trailing brackets.
pub fn remove_brackets(s: &str) -> String {
    let s = s.trim();
    let mut result = String::from(s);
    if s.starts_with(&['(', '[', '{', '<', '【']) {
        result.remove(0);
    }
    if s.ends_with(&[')', ']', '}', '>', '】']) {
        result.pop();
    }
    String::from(result.trim())
}

/// Remove all pairs of matching empty brackets.
pub fn remove_empty_brackets(s: String) -> String {
    let mut result = String::new();

    let mut iter = s.chars();
    let mut a = iter.next();
    let mut b = iter.next();
    let mut skip = false;

    loop {
        if a.is_none() {
            break;
        }
        if !skip {
            result.push(a.unwrap());
        }

        if b.is_none() {
            break;
        }
        result.push(b.unwrap());

        skip = false;
        for (left, right) in [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')] {
            if a == Some(left) && b == Some(right) {
                skip = true;
                break;
            }
        }

        if skip {
            result.pop();
            result.pop();
            a = iter.next();
            b = iter.next();
        } else {
            a = b;
            b = iter.next();
        }
        skip = !skip;
    }

    if result.len() < s.len() {
        remove_empty_brackets(result)
    } else {
        result // Nothing more to remove
    }
}

/// Remove all duplicate whitespace.
pub fn remove_duplicate_whitespace(s: String) -> String {
    let mut result = String::new();

    let mut previous_space = false;
    for c in s.chars() {
        if c == ' ' && previous_space {
            continue;
        }
        previous_space = c == ' ';
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_brackets() {
        let inputs = [
            ("(official video)", "official video"),
            ("[hard remix]", "hard remix"),
            ("{instrumental}", "instrumental"),
            ("<remix>", "remix"),
            ("( extended mix )", "extended mix"),
            ("【mix】", "mix"),
            (" [remix]", "remix"),
            (" [ remix ]  ", "remix"),
            ("(OFFICIAL MUSIC VIDEO 🎵)", "OFFICIAL MUSIC VIDEO 🎵"),
        ];
        for (input, expected) in inputs {
            assert_eq!(remove_brackets(input), expected);
        }
    }

    #[test]
    fn removes_str_from_string() {
        let inputs = [
            ("Official HD Video", "HD", "Official  Video"),
            ("03. Artist - Song", "03.", "Artist - Song"),
            ("A ➕ B", "B", "A ➕"),
            ("A ➕ B", "➕", "A  B"),
        ];
        for (input, to_remove, expected) in inputs {
            assert_eq!(
                remove_str_from_string(input.to_string(), to_remove),
                expected
            );
        }
    }

    #[test]
    fn removes_empty_brackets() {
        let inputs = [
            ("", ""),
            ("(", "("),
            (")", ")"),
            ("()", ""),
            ("[]", ""),
            ("{}", ""),
            ("<>", ""),
            ("(<>)[]", ""),
            ("[(()<{}>)[]((()))]", ""),
            ("Song ()", "Song "),
            ("Song 🎵 []", "Song 🎵 "),
        ];
        for (input, expected) in inputs {
            assert_eq!(remove_empty_brackets(input.to_string()), expected);
        }
    }

    #[test]
    fn removes_duplicate_whitespace() {
        let inputs = [
            ("  ", " "),
            ("a  b", "a b"),
            ("a   b", "a b"),
            ("a  bc  d", "a bc d"),
            ("Song  🎵", "Song 🎵"),
            ("🎵  Song", "🎵 Song"),
        ];
        for (input, expected) in inputs {
            assert_eq!(remove_duplicate_whitespace(input.to_string()), expected);
        }
    }
}
