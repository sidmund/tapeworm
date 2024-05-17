mod common;

use common::{run, setup};
use dirs;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[test]
fn shows_list() {
    let config = setup(vec!["list"]).unwrap();
    run(config).unwrap();
}

#[test]
fn fails_without_library() {
    let commands = ["show", "add", "download", "tag", "deposit", "process"];
    for command in commands {
        let config = setup(vec![command]);
        assert!(config.is_err());
    }
}

#[test]
fn fails_with_non_existing_library() {
    assert!(setup(vec!["show", "tw-test-gaf9843uj3nrj"]).is_err());
    assert!(setup(vec!["download", "tw-test-i1g491osf"]).is_err());
    assert!(setup(vec!["tag", "tw-test-a98w46yfha0huf"]).is_err());
    assert!(setup(vec!["deposit", "tw-test-9732tryafo"]).is_err());
    assert!(setup(vec!["process", "tw-test-aiyeq29a48"]).is_err());
}

#[test]
fn add_fails_without_args() {
    let config = setup(vec!["add", "tw-test-aisy822rit"]);
    assert!(config.is_err());
}

#[test]
fn shows_library() {
    let test_lib = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join("tw-test-show");
    fs::create_dir_all(&test_lib).unwrap();

    let config = setup(vec!["show", "tw-test-show"]).unwrap();
    run(config).unwrap();

    fs::remove_dir_all(&test_lib).unwrap(); // cleanup
}

#[test]
fn adds_url_to_library() {
    let config = setup(vec![
        "add",
        "tw-test-url",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    ])
    .unwrap();
    let lib_path = config.lib_path.clone().unwrap();
    let input_path = config.input_path.clone().unwrap();
    run(config).unwrap();

    let contents = fs::read_to_string(input_path).unwrap();
    assert_eq!("https://www.youtube.com/watch?v=dQw4w9WgXcQ\n", contents);

    fs::remove_dir_all(&lib_path).unwrap(); // cleanup
}

#[test]
fn adds_term_to_library() {
    let config = setup(vec!["add", "tw-test-term", "Darude", "Sandstorm"]).unwrap();
    let lib_path = config.lib_path.clone().unwrap();
    let input_path = config.input_path.clone().unwrap();
    run(config).unwrap();

    let contents = fs::read_to_string(input_path).unwrap();
    assert_eq!("ytsearch:\"Darude Sandstorm\"\n", contents);

    fs::remove_dir_all(&lib_path).unwrap(); // cleanup
}

fn download(lib: &str, clear_input: bool) {
    // Write some yt-dlp options
    let lib_path = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join(lib);
    fs::create_dir_all(&lib_path).unwrap();

    let mut conf = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lib_path.join("yt-dlp.conf"))
        .unwrap();
    let contents = format!(
        "-i -P \"{}\" -o \"%(title)s.%(ext)s\" -x --audio-format mp3",
        lib_path.to_str().unwrap()
    );
    conf.write_all(contents.as_bytes()).unwrap();

    // Add a query
    let config = setup(vec!["add", lib, "Darude", "Sandstorm"]).unwrap();
    run(config).unwrap();

    // Wait for download
    let config = if clear_input {
        setup(vec!["download", lib, "-c"]).unwrap()
    } else {
        setup(vec!["download", lib]).unwrap()
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
        let contents = fs::read_to_string(input_path).unwrap();
        assert!(contents.is_empty());
    } else {
        let contents = fs::read_to_string(input_path).unwrap();
        assert_eq!("ytsearch:\"Darude Sandstorm\"\n", contents);
    }

    fs::remove_dir_all(&lib_path).unwrap(); // cleanup
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
    let lib_path = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join(lib);
    fs::create_dir_all(&lib_path).unwrap();

    assert!(run(setup(vec!["tag", lib]).unwrap()).is_err());
    assert!(run(setup(vec!["tag", lib, "-i"]).unwrap()).is_err());
    assert!(run(setup(vec!["tag", lib, "-i", "tw-test-uy4hfaif"]).unwrap()).is_err());

    fs::remove_dir_all(&lib_path).unwrap(); // cleanup
}

#[test]
#[ignore]
fn tags() {
    // Create the library dir
    let lib = "tw-test-tag";
    let lib_path = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join(lib);
    fs::create_dir_all(&lib_path).unwrap();

    // Does not fail if no files are present
    run(setup(vec!["tag", lib, "-i", lib_path.to_str().unwrap()]).unwrap()).unwrap();

    // Copy an untagged resource file to the lib dir
    let res_path = common::get_resources();
    let mp3_path = res_path.join("rickroll.mp3");
    fs::copy(mp3_path, lib_path.join("rickroll.mp3")).unwrap();

    // TODO how to pass input so we dont have to interact with the test?
    // I think i'll just make a cli/lib.conf option for it
    run(setup(vec!["tag", lib, "-i", lib_path.to_str().unwrap()]).unwrap()).unwrap();

    // TODO verify that mp3 has at least the title tag
    // TODO test tag::tag (whatever is not covered by unit tests)
    // TODO verify that the tags were updated

    fs::remove_dir_all(&lib_path).unwrap(); // cleanup
}

#[test]
fn fails_deposit_on_incorrect_args() {
    let lib = "tw-test-deposit-fail";
    let lib_path = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join(lib);
    fs::create_dir_all(&lib_path).unwrap();
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
                let mut args = vec!["deposit", lib];
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

    fs::remove_dir_all(&lib_path).unwrap(); // cleanup
}

#[test]
fn deposits() {}

#[test]
fn fails_to_process_without_steps() {
    let test_lib = PathBuf::from(dirs::config_dir().unwrap())
        .join("tapeworm")
        .join("tw-test-no-steps");
    fs::create_dir_all(&test_lib).unwrap();

    let config = setup(vec!["process", "tw-test-no-steps"]).unwrap();
    assert!(run(config).is_err());

    fs::remove_dir_all(&test_lib).unwrap(); // cleanup
}