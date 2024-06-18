mod common;

use audiotags::Tag;
use chrono::{Datelike, Utc};
use common::*;
use std::{fs, io::BufReader, path::PathBuf};

#[test]
fn runs_without_command_or_library() {
    run(build(vec![]).unwrap()).unwrap();
}

#[test]
fn runs_non_library_commands() {
    for cmd in ["help", "h", "-h", "--help", "list", "ls", "l"] {
        run(build(vec![cmd]).unwrap()).unwrap();
    }
}

/// Assumes that the test is not run inside a library folder (no `.tapeworm` subfolder)
#[test]
fn library_commands_fail_without_library() {
    for cmd in ["show", "add", "download", "tag", "deposit", "process"] {
        assert!(build(vec![cmd]).is_err());
    }
}

/// Test that tapeworm fails when:
/// - The alias does not exist
/// - The library path does not exist
/// - The library at the path does not have a ".tapeworm" config folder
#[test]
fn fails_with_non_existing_library() {
    for cmd in ["show", "add", "download", "tag", "deposit", "process"] {
        // Non-existing alias fails
        let alias = format!("{}-not-an-alias", cmd);
        assert!(build(vec![&alias, cmd]).is_err());

        // Just the base directory without a ".tapeworm" config folder should be an invalid library
        let lib = Library::new().create_base_folder();
        assert!(build(vec![lib.arg(), cmd]).is_err());

        // Non-existing path fails
        let lib = Library::new();
        assert!(build(vec![lib.arg(), cmd]).is_err());
    }
}

#[test]
fn shows_library() {
    let lib = Library::new().create_cfg_folder();
    run(build(vec![lib.arg()]).unwrap()).unwrap();
    run(build(vec![lib.arg(), "show"]).unwrap()).unwrap();
}

#[test]
fn alias() {
    let lib = Library::new().create_cfg_folder();
    let alias = format!("{}-alias", lib.name);

    // Errors when alias does not exist
    assert!(build(vec![&alias, "show"]).is_err());

    // Succeeds after alias is added
    run(build(vec![lib.arg(), "alias", &alias]).unwrap()).unwrap();
    run(build(vec![&alias, "alias"]).unwrap()).unwrap();
    run(build(vec![&alias, "show"]).unwrap()).unwrap();

    // Remove the alias, either lib path or alias should work
    run(build(vec![&alias, "alias", "-r"]).unwrap()).unwrap();
    run(build(vec![lib.arg(), "alias", "-r"]).unwrap()).unwrap();

    // Errors again when alias has been removed
    assert!(build(vec![&alias, "show"]).is_err());
}

#[test]
fn clean_removes_empty_directories() {
    let lib = Library::new().create_cfg_folder();
    let folders = [
        (lib.base_dir.join("f1"), false),
        (lib.base_dir.join("f2"), true),
        (lib.base_dir.join("f3"), false),
        (lib.base_dir.join("f3").join("f4"), false),
        (lib.base_dir.join("f5"), true),
        (lib.base_dir.join("f5").join("f6"), false),
    ];
    let files = [
        lib.base_dir.join("f2").join("file"),
        lib.base_dir.join("f5").join("file"),
    ];
    for (folder, _) in &folders {
        fs::create_dir_all(folder).unwrap();
        assert!(fs::metadata(folder).is_ok());
    }
    for file in &files {
        write(file, String::from("test"));
        assert!(fs::metadata(file).is_ok());
    }

    run(build(vec![lib.arg(), "clean"]).unwrap()).unwrap();
    for (folder, keep) in &folders {
        if *keep {
            assert!(fs::metadata(folder).is_ok());
        } else {
            assert!(fs::metadata(folder).is_err());
        }
    }
    for file in &files {
        assert!(fs::metadata(file).is_ok());
    }
}

#[test]
fn add_fails_without_args() {
    let lib = Library::new().create_cfg_folder();
    assert!(build(vec![lib.arg(), "add"]).is_err());
}

#[test]
fn adds_to_library() {
    let lib = Library::new().create_in_out_folders();
    let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
    let config = build(vec![lib.arg(), "add", url]).unwrap();
    let input_txt = config.input_path.clone().unwrap();
    run(config).unwrap();
    run(build(vec![lib.arg(), "add", "Darude Sandstorm"]).unwrap()).unwrap();

    assert_eq!(
        format!("{}\nytsearch:Darude Sandstorm\n", url),
        read(&input_txt)
    );
}

fn download(clear_input: bool) {
    let lib = Library::new().create_in_out_folders();

    // Write some yt-dlp options
    let options = format!(
        "-i -P \"{}\" -o \"%(title)s.%(ext)s\" -x --audio-format mp3",
        lib.input_arg()
    );
    write(&lib.cfg_dir.join("yt-dlp.conf"), options);

    // Add a query
    run(build(vec![lib.arg(), "add", "Darude Sandstorm"]).unwrap()).unwrap();

    // Verify that the download folder is empty
    assert_eq!(0, fs::read_dir(&lib.input_dir).unwrap().count());

    // Wait for download
    let config = if clear_input {
        build(vec![lib.arg(), "download", "-ac"]).unwrap()
    } else {
        build(vec![lib.arg(), "download", "-a"]).unwrap()
    };
    let input_path = config.input_path.clone().unwrap();
    let clear_input = config.clear_input;
    run(config).unwrap();

    // Verify that the downloaded file is present
    assert_eq!(1, fs::read_dir(&lib.input_dir).unwrap().count());

    if clear_input {
        assert!(read(&input_path).is_empty());
    } else {
        assert_eq!("ytsearch:Darude Sandstorm\n", read(&input_path));
    }
}

#[test]
#[ignore]
fn downloads_and_keeps_input() {
    download(false);
}

#[test]
#[ignore]
fn downloads_and_clears_input() {
    download(true);
}

#[test]
fn fails_tag_on_incorrect_args() {
    let lib = Library::new().create_cfg_folder();
    assert!(build(vec![lib.arg(), "tag"]).is_err());
    assert!(build(vec![lib.arg(), "tag", "-i"]).is_err());
    assert!(build(vec![lib.arg(), "tag", "-i", "uy4hfaif"]).is_err());
}

#[test]
fn tag_does_not_fail_without_files() {
    let lib = Library::new().create_in_out_folders();
    run(build(vec![lib.arg(), "tag", "-i", lib.input_arg()]).unwrap()).unwrap();
}

#[test]
fn tag_skips_unsupported_files() {
    let lib = Library::new().create_in_out_folders();

    let files = [
        "empty_title.mp3",
        "no_tags.mp3",
        "no_title.mp3",
        "not_audio.jpg",
    ];
    for file in files {
        lib.copy_to_input(file);
    }

    run(build(vec![lib.arg(), "tag", "-i", lib.input_arg()]).unwrap()).unwrap();
}

fn test_tags(original: &PathBuf, expected: &PathBuf, title: Option<&str>, artist: Option<&str>) {
    assert!(fs::metadata(original).is_err());
    let tag = Tag::new().read_from_path(expected).unwrap();
    assert_eq!(tag.title(), title);
    assert_eq!(tag.artist(), artist);
}

fn tag(ext: &str, auto_tag: bool) {
    let lib = Library::new().create_in_out_folders();

    let file = format!("title.{}", ext);
    lib.copy_to_input(&file);

    let old = lib.input_dir.join(&file);
    let mut new = lib.input_dir.join("Artist - Song [Radio Edit]");
    new.set_extension(ext);
    test_tags(&new, &old, Some("Artist - Song (Radio Edit)"), None);

    if auto_tag {
        run(build(vec![lib.arg(), "tag", "-ti", lib.input_arg()]).unwrap()).unwrap();
    } else {
        let buffer = Vec::from(b"y\n");
        let reader: BufReader<&[u8]> = BufReader::new(buffer.as_ref());
        let config = build(vec![lib.arg(), "tag", "-i", lib.input_arg()]).unwrap();
        run_with(config, reader).unwrap();
    }
    test_tags(&old, &new, Some("Song [Radio Edit]"), Some("Artist"));
}

#[test]
fn tags_diverse_audio_formats_with_title_tag() {
    tag("mp3", false);
    tag("flac", false);
    tag("mp3", true);
}

#[test]
fn cancel_tagging_preserves_file() {
    let lib = Library::new().create_in_out_folders();
    lib.copy_to_input("title.mp3");

    let old = lib.input_dir.join("title.mp3");
    let new = lib.input_dir.join("Artist - Song [Radio Edit].mp3");
    test_tags(&new, &old, Some("Artist - Song (Radio Edit)"), None);

    let buffer = Vec::from(b"n\n");
    let reader: BufReader<&[u8]> = BufReader::new(buffer.as_ref());
    let config = build(vec![lib.arg(), "tag", "-i", lib.input_arg()]).unwrap();
    run_with(config, reader).unwrap();
    test_tags(&new, &old, Some("Artist - Song (Radio Edit)"), None);
}

#[test]
fn fails_deposit_on_incorrect_args() {
    let lib = Library::new().create_in_out_folders();

    // Values are: Omit the option, No value for option, Invalid value, Valid value
    let i_opts = [None, Some(""), Some("iiii"), Some(lib.input_arg())];
    let o_opts = [None, Some(""), Some(lib.output_arg())];
    let d_opts = [None, Some(""), Some("dddd"), Some("A-Z")];

    // Test each permutation of options
    for i in i_opts {
        for o in o_opts {
            for d in d_opts {
                let mut args = vec![lib.arg(), "deposit"];
                // TODO also shuffle their order (6 different ways)
                if let Some(i) = i {
                    args.push("-i");
                    if !i.is_empty() {
                        args.push(i);
                    }
                }
                if let Some(o) = o {
                    args.push("-o");
                    if !o.is_empty() {
                        args.push(o);
                    }
                }
                if let Some(d) = d {
                    args.push("-d");
                    if !d.is_empty() {
                        args.push(d);
                    }
                }

                // Either fail during config or during run
                if let Ok(cfg) = build(args) {
                    // Succeed only with (not in order):
                    // -i lib_path -o any
                    // -i lib_path -o any -d A-Z
                    if cfg.input_dir.as_ref().is_some_and(|s| s == &lib.input_dir)
                        && cfg.target_dir.as_ref().is_some()
                    {
                        run(cfg).unwrap();
                        continue;
                    }
                    assert!(run(cfg).is_err());
                } else {
                    assert!(true);
                }
            }
        }
    }
}

fn deposit(mode: &str, filename: &str, az_path: &PathBuf, date_path: &PathBuf) {
    let lib = Library::new().create_in_out_folders();
    lib.copy_to_input(filename);

    let original_path = lib.input_dir.join(filename);
    let drop_path = lib.output_dir.join(filename);
    let az_path = lib.output_dir.join(az_path).join(filename);
    let date_path = lib.output_dir.join(date_path).join(filename);
    assert!(fs::metadata(&drop_path).is_err());
    assert!(fs::metadata(&az_path).is_err());
    assert!(fs::metadata(&date_path).is_err());

    let i = lib.input_arg();
    let o = lib.output_arg();
    let opts = match mode {
        "A-Z" => vec![lib.arg(), "deposit", "-i", i, "-o", o, "-d", "A-Z"],
        "DATE" => vec![lib.arg(), "deposit", "-i", i, "-o", o, "-d", "DATE"],
        _ => vec![lib.arg(), "deposit", "-i", i, "-o", o],
    };
    run(build(opts).unwrap()).unwrap();

    assert!(fs::metadata(original_path).is_err());
    match mode {
        "A-Z" => {
            assert!(fs::metadata(drop_path).is_err());
            assert!(fs::metadata(az_path).is_ok());
            assert!(fs::metadata(date_path).is_err());
        }
        "DATE" => {
            assert!(fs::metadata(drop_path).is_err());
            assert!(fs::metadata(az_path).is_err());
            assert!(fs::metadata(date_path).is_ok());
        }
        _ => {
            assert!(fs::metadata(drop_path).is_ok());
            assert!(fs::metadata(az_path).is_err());
            assert!(fs::metadata(date_path).is_err());
        }
    }
}

#[test]
fn deposits() {
    // Files are copied from resources/test, so their date will be today
    let today = Utc::now();
    let today = PathBuf::from(today.year().to_string()).join(format!("{:02}", today.month()));

    let files = [
        ("no_tags.mp3", PathBuf::from("N"), &today),
        ("tagged.mp3", PathBuf::from("A").join("Artist"), &today),
        (
            "tagged_album.mp3",
            PathBuf::from("A").join("Artist").join("Album"),
            &today,
        ),
    ];
    for (filename, az_path, date_path) in files {
        for drop in ["A-Z", "DATE", "x"] {
            deposit(drop, filename, &az_path, &date_path);
        }
    }
}

#[test]
fn fails_to_process_without_steps() {
    let lib = Library::new().create_cfg_folder();
    assert!(build(vec![lib.arg(), "process"]).is_err());
}

#[test]
fn fails_to_process_illegal_commands() {
    let lib = Library::new().create_cfg_folder();
    assert!(build(vec![lib.arg(), "process", "-s", "add"]).is_err());
    assert!(build(vec![lib.arg(), "process", "-s", "process"]).is_err());
    assert!(build(vec![lib.arg(), "process", "-s", "list,process"]).is_err());
}
