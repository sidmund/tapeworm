mod common;

use audiotags::Tag;
use common::*;
use std::{fs, io::BufReader, path::PathBuf};

#[test]
fn runs_without_command_or_library() {
    run(setup(vec![]).unwrap()).unwrap();
}

#[test]
fn runs_non_library_commands() {
    for cmd in ["help", "h", "-h", "--help", "list", "ls", "l"] {
        run(setup(vec![cmd]).unwrap()).unwrap();
    }
}

#[test]
fn library_commands_fail_without_library() {
    for cmd in ["show", "add", "download", "tag", "deposit", "process"] {
        assert!(setup(vec![cmd]).is_err());
    }
}

#[test]
fn fails_with_non_existing_library() {
    for cmd in ["show", "download", "tag", "deposit", "process"] {
        let lib = format!("tw-test-{}-unexist", cmd);
        assert!(setup(vec![&lib, cmd]).is_err());
    }
}

#[test]
fn add_fails_without_args() {
    assert!(setup(vec!["add", "tw-test-aisy822rit"]).is_err());
}

#[test]
fn shows_library() {
    let lib = "tw-test-show";
    let lib_path = create_lib(lib);
    run(setup(vec![lib]).unwrap()).unwrap();
    run(setup(vec![lib, "show"]).unwrap()).unwrap();
    destroy(lib_path);
}

#[test]
fn adds_to_library() {
    let lib = "tw-test-add";
    let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
    let config = setup(vec![lib, "add", url]).unwrap();
    let lib_path = config.lib_path.clone().unwrap();
    let input_path = config.input_path.clone().unwrap();
    run(config).unwrap();
    run(setup(vec![lib, "add", "Darude Sandstorm"]).unwrap()).unwrap();

    assert_eq!(
        format!("{}\nytsearch:Darude Sandstorm\n", url),
        read(input_path)
    );

    destroy(lib_path);
}

fn download(lib: &str, clear_input: bool) {
    let lib_path = create_lib(lib);

    // Write some yt-dlp options
    let options = format!(
        "-i -P \"{}\" -o \"%(title)s.%(ext)s\" -x --audio-format mp3",
        lib_path.to_str().unwrap()
    );
    write(lib_path.join("yt-dlp.conf"), options);

    // Add a query
    run(setup(vec![lib, "add", "Darude Sandstorm"]).unwrap()).unwrap();

    // Wait for download
    let config = if clear_input {
        setup(vec![lib, "download", "-c"]).unwrap()
    } else {
        setup(vec![lib, "download"]).unwrap()
    };
    let lib_path = config.lib_path.clone().unwrap();
    let input_path = config.input_path.clone().unwrap();
    let clear_input = config.clear_input;
    run(config).unwrap();

    // Verify that only the input.txt, yt-dlp.conf, and downloaded mp3 exist
    let mut count = 0;
    for entry in fs::read_dir(&lib_path).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        let filename = entry.file_name().to_str().unwrap().to_string();
        assert!(
            filename.eq("input.txt") || filename.eq("yt-dlp.conf") || filename.ends_with(".mp3")
        );
        count += 1;
    }
    assert_eq!(3, count);

    if clear_input {
        assert!(read(input_path).is_empty());
    } else {
        assert_eq!("ytsearch:Darude Sandstorm\n", read(input_path));
    }

    destroy(lib_path);
}

#[test]
#[ignore]
fn downloads_and_keeps_input() {
    download("tw-test-dl-keep", false);
}

#[test]
#[ignore]
fn downloads_and_clears_input() {
    download("tw-test-dl-clear", true);
}

#[test]
fn fails_tag_on_incorrect_args() {
    let lib = "tw-test-tag-fail";
    let lib_path = create_lib(lib);
    assert!(run(setup(vec![lib, "tag"]).unwrap()).is_err());
    assert!(run(setup(vec![lib, "tag", "-i"]).unwrap()).is_err());
    assert!(run(setup(vec![lib, "tag", "-i", "tw-test-uy4hfaif"]).unwrap()).is_err());
    destroy(lib_path);
}

#[test]
fn tag_does_not_fail_without_files() {
    let lib = "tw-test-tag-no-files";
    let lib_path = create_lib(lib);
    run(setup(vec![lib, "tag", "-i", lib_path.to_str().unwrap()]).unwrap()).unwrap();
    destroy(lib_path);
}

#[test]
fn tag_skips_unsupported_files() {
    let lib = "tw-test-tag-unsupported";
    let lib_path = create_lib(lib);

    let files = [
        "empty_title.mp3",
        "no_tags.mp3",
        "no_title.mp3",
        "not_audio.jpg",
    ];
    for file in files {
        copy(file, &lib_path);
    }

    run(setup(vec![lib, "tag", "-i", lib_path.to_str().unwrap()]).unwrap()).unwrap();

    destroy(lib_path);
}

fn tag(ext: &str) {
    let lib = "tw-test-tag-yes";
    let lib_path = create_lib(lib);

    let file = format!("title.{}", ext);
    copy(&file, &lib_path);

    let mut expected = lib_path.join("Artist - Song [Radio Edit]");
    expected.set_extension(ext);

    assert!(fs::metadata(&expected).is_err());
    let tag = Tag::new().read_from_path(lib_path.join(&file)).unwrap();
    assert_eq!(tag.title().unwrap(), "Artist - Song (Radio Edit)");
    assert_eq!(tag.artist(), None);

    let buffer = Vec::from(b"y\n");
    let reader: BufReader<&[u8]> = BufReader::new(buffer.as_ref());
    let config = setup(vec![lib, "tag", "-i", lib_path.to_str().unwrap()]).unwrap();
    run_with(config, reader).unwrap();

    assert!(fs::metadata(lib_path.join(&file)).is_err());
    let tag = Tag::new().read_from_path(&expected).unwrap();
    assert_eq!(tag.title().unwrap(), "Song [Radio Edit]");
    assert_eq!(tag.artist().unwrap(), "Artist");

    destroy(lib_path);
}

#[test]
fn tags_diverse_audio_formats_with_title_tag() {
    tag("mp3");
    tag("flac");
}

#[test]
fn cancel_tagging_preserves_file() {
    let lib = "tw-test-tag-no";
    let lib_path = create_lib(lib);

    copy("title.mp3", &lib_path);

    assert!(fs::metadata(lib_path.join("Artist - Song [Radio Edit].mp3")).is_err());
    let tag = Tag::new()
        .read_from_path(lib_path.join("title.mp3"))
        .unwrap();
    assert_eq!(tag.title().unwrap(), "Artist - Song (Radio Edit)");
    assert_eq!(tag.artist(), None);

    let buffer = Vec::from(b"n\n");
    let reader: BufReader<&[u8]> = BufReader::new(buffer.as_ref());
    let config = setup(vec![lib, "tag", "-i", lib_path.to_str().unwrap()]).unwrap();
    run_with(config, reader).unwrap();

    assert!(fs::metadata(lib_path.join("Artist - Song [Radio Edit].mp3")).is_err());
    let tag = Tag::new()
        .read_from_path(lib_path.join("title.mp3"))
        .unwrap();
    assert_eq!(tag.title().unwrap(), "Artist - Song (Radio Edit)");
    assert_eq!(tag.artist(), None);

    destroy(lib_path);
}

#[test]
fn fails_deposit_on_incorrect_args() {
    let lib = "tw-test-deposit-fail";
    let lib_path = create_lib(lib);
    let lib_str = lib_path.to_str().unwrap();

    // Values are: Omit the option, No value for option, Invalid value, Valid value
    let i_opts = [None, Some(""), Some("tw-test-iiii"), Some(lib_str)];
    let o_opts = [None, Some(""), Some(lib_str)]; // An invalid path does not exist here, as it
                                                  // will be created
    let d_opts = [None, Some(""), Some("dddd"), Some("A-Z")];

    // Test each permutation of options
    for i in i_opts {
        for o in o_opts {
            for d in d_opts {
                let mut args = vec![lib, "deposit"];
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
                if let Ok(config) = setup(args) {
                    // Succeed only with (not in order):
                    // -i lib_path -o any
                    // -i lib_path -o any -d A-Z
                    if config.input_dir.as_ref().is_some_and(|s| s == &lib_path)
                        && config.target_dir.as_ref().is_some()
                    {
                        let org = config.organize.as_ref();
                        if org.is_none() || org.unwrap() == "A-Z" {
                            run(config).unwrap();
                            continue;
                        }
                    }
                    assert!(run(config).is_err());
                } else {
                    assert!(true);
                }
            }
        }
    }

    destroy(lib_path);
}

fn deposit(lib: &str, mode: &str, filename: &str, az_path: &PathBuf, date_path: &PathBuf) {
    let (lib_path, lib_in, lib_out) = create_lib_with_folders(lib);

    copy(filename, &lib_in);

    let original_path = lib_in.join(filename);
    let drop_path = lib_out.join(filename);
    let az_path = lib_out.join(az_path).join(filename);
    let date_path = lib_out.join(date_path).join(filename);
    assert!(fs::metadata(&drop_path).is_err());
    assert!(fs::metadata(&az_path).is_err());
    assert!(fs::metadata(&date_path).is_err());

    let i = lib_in.to_str().unwrap();
    let o = lib_out.to_str().unwrap();
    let opts = match mode {
        "A-Z" => vec![lib, "deposit", "-i", i, "-o", o, "-d", "A-Z"],
        "DATE" => vec![lib, "deposit", "-i", i, "-o", o, "-d", "DATE"],
        _ => vec![lib, "deposit", "-i", i, "-o", o],
    };
    run(setup(opts).unwrap()).unwrap();

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

    destroy(lib_path);
}

#[test]
fn deposits() {
    let files = [
        (
            "no_tags.mp3",
            PathBuf::from("N"),
            PathBuf::from("2024").join("05"),
        ),
        (
            "tagged.mp3",
            PathBuf::from("A").join("Artist"),
            PathBuf::from("2024").join("05"),
        ),
        (
            "tagged_album.mp3",
            PathBuf::from("A").join("Artist").join("Album"),
            PathBuf::from("2024").join("05"),
        ),
    ];
    for (filename, az_path, date_path) in files {
        for drop in ["A-Z", "DATE", "x"] {
            deposit("tw-test-deposit", drop, filename, &az_path, &date_path);
        }
    }
}

#[test]
fn fails_to_process_without_steps() {
    let lib = "tw-test-no-steps";
    let lib_path = create_lib(lib);
    assert!(run(setup(vec![lib, "process"]).unwrap()).is_err());
    destroy(lib_path);
}

#[test]
fn fails_to_process_illegal_commands() {
    let lib = "tw-test-illegal";
    let lib_path = create_lib(lib);
    assert!(run(setup(vec![lib, "process", "-s", "add"]).unwrap()).is_err());
    assert!(run(setup(vec![lib, "process", "-s", "process"]).unwrap()).is_err());
    assert!(run(setup(vec![lib, "process", "-s", "list,process"]).unwrap()).is_err());
    destroy(lib_path);
}
