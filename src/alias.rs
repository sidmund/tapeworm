use crate::{types, util, Config};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn run(config: &Config) -> types::UnitResult {
    if config.terms.is_none() {
        show_aliases(config); // `tapeworm ALIAS_OR_PATH alias`
        return Ok(());
    }

    let mut new_aliases = config.aliases.clone();
    let remove_or_alias = config.terms.as_ref().unwrap().get(0).unwrap();
    if remove_or_alias == "-r" {
        // When invoking `tapeworm ALIAS alias -r`, remove just that ALIAS
        if !remove_alias(&mut new_aliases, &config.lib_alias) {
            // When invoking `tapeworm LIB_PATH alias -r`, remove all aliases for LIB_PATH
            remove_aliases_for_path(&mut new_aliases, config.lib_path.as_ref().unwrap());
        }
    } else {
        // When invoking `tapeworm ALIAS_OR_PATH alias ALIAS`
        let alias = remove_or_alias.to_owned();
        let path = config.lib_path.clone().unwrap();
        add_alias(&mut new_aliases, &config.lib_alias, alias, path);
    }
    write(new_aliases, &config.general_conf)
}

fn show_aliases(config: &Config) {
    if config.lib_alias.is_some() {
        // Print the path the alias points to
        println!("{}", config.lib_path.as_ref().unwrap().display());
    } else {
        // Print the aliases setup for the lib_path
        for (alias, path) in &config.aliases {
            if path == config.lib_path.as_ref().unwrap() {
                println!("{}", alias);
            }
        }
    }
}

fn write(aliases: BTreeMap<String, PathBuf>, path: &PathBuf) -> types::UnitResult {
    let content = aliases.iter().fold(String::new(), |acc, (alias, path)| {
        format!("{}{}={}\n", acc, alias, path.to_str().unwrap())
    });
    println!("To write: {content}");
    util::write(path, content)
}

/// Adds the `alias` for `path`. If `old_alias` is defined, that alias will be removed first.
fn add_alias(
    aliases: &mut BTreeMap<String, PathBuf>,
    old_alias: &Option<String>,
    alias: String,
    path: PathBuf,
) {
    remove_alias(aliases, old_alias);
    aliases.insert(alias, path);
}

fn remove_alias(aliases: &mut BTreeMap<String, PathBuf>, alias: &Option<String>) -> bool {
    if let Some(alias) = alias {
        aliases.remove(alias);
        true
    } else {
        false
    }
}

fn remove_aliases_for_path(aliases: &mut BTreeMap<String, PathBuf>, path: &PathBuf) {
    let to_remove = aliases
        .iter()
        .filter(|(_, p)| *p == path)
        .map(|(alias, _)| alias.to_owned())
        .collect::<Vec<String>>();
    to_remove.iter().for_each(|alias| {
        aliases.remove(alias);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwrites_alias() {
        let mut aliases = BTreeMap::new();
        add_alias(
            &mut aliases,
            &None,
            String::from("test"),
            PathBuf::from("test/library"),
        );
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("test"), Some(&PathBuf::from("test/library")));

        add_alias(
            &mut aliases,
            &Some(String::from("test")),
            String::from("test2"),
            PathBuf::from("test/library"),
        );
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("test2"), Some(&PathBuf::from("test/library")));
    }

    #[test]
    fn removes_aliases_for_path() {
        let mut aliases = BTreeMap::new();
        aliases.insert(String::from("test"), PathBuf::from("test/library"));
        aliases.insert(String::from("alt"), PathBuf::from("alt/library"));
        aliases.insert(String::from("test2"), PathBuf::from("test/library"));

        remove_aliases_for_path(&mut aliases, &PathBuf::from("test/library"));
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("alt"), Some(&PathBuf::from("alt/library")));
    }
}
