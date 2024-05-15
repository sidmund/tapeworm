# tapeworm

tapeworm is a media file processor written in Rust. Its features are:

- Scrape websites for URLs or queries, see [supported websites](#supported-websites-for-scraping)
- Download (scraped) URLs and queries
- Manage different yt-dlp configurations
- Extract additional tags from the `title` tag (only for music/video)
- Organize downloaded/tagged files into directories

tapeworm uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) for downloading and can download whatever yt-dlp can download.

## Is this for you?

If you just need to download URL(s), use yt-dlp. yt-dlp has options for specifying an input file and configuration files. yt-dlp also works with queries like `yt-dlp ytsearch:"query"`. If that is not enough and you need some of the following features, tapeworm is for you:

- You want to obtain URLs/queries from sites not supported by yt-dlp, e.g. yt-dlp cannot download from Spotify; but tapeworm can scrape Spotify for song information and will download the songs using `ytsearch` queries
- You want a single application to store URLs, to download them, to tag them, and to organize them
- You want to setup different download options for different sets of input URLs, and be able to easily configure and invoke them. E.g. you have a music and a video library and want a single application to easily download sources for them with the right options
- You like the abstraction tapeworm provides by never having to specify the config file yourself, or worrying about what file to store URLs in, as this can all be done with simple tapeworm commands

## Build

A Rust installation is required. tapeworm compiles with Rust 1.74.0+ (stable).

To build tapeworm:
```sh
git clone https://github.com/sidmund/tapeworm
cd tapeworm
cargo build --release
./target/release/tapeworm help
```

## Usage

tapeworm works with "libraries". A library specifies a.o. what files to download, how to download them, and how to process them. Commonly, libraries are used for downloading music/video. However, you can also use libraries just for tagging files, or organizing them; and skip the download functionality altogether.

Minimal example setup and usage when you want to use tapeworm for downloading:

```sh
# Create the library by recording the first query
tapeworm add LIBRARY the artist - a song # records 'ytsearch:"the artist - a song"'
tapeworm add LIBRARY https://youtube.com/watch?v=123 # records the URL

# Download all URLs/queries
tapeworm download LIBRARY

# Optionally, tag and deposit the downloaded files
tapeworm tag LIBRARY # extract more tags from "title"
tapeworm deposit LIBRARY # organize into a target folder
```

If you add a URL from a [scraping supported site](#supported-websites-for-scraping), tapeworm will scrape that page to find song information and add that as a `ytsearch` query to the library.

Downloading the library will first download each input (whether URL or query), and may then process the downloaded files further, e.g. tagging audio files.

If you want to use tapeworm merely for tagging and/or organizing files, see the following minimal example setup and usage:

```sh
tapeworm tag LIBRARY # `INPUT_DIR` should be set, and have some files in it
tapeworm deposit LIBRARY # Both `INPUT_DIR` and `TARGET_DIR` should be set
```

### Configuration

The library configuration determines the behavior of each tapeworm command.

The config directory shall refer to one of the following paths (depending on your system):

- Unix: `/home/USER/.config/tapeworm/LIBRARY/`
- Windows: `/c/Users/USER/AppData/Roaming/tapeworm/LIBRARY/`

tapeworm will try to find the following files in this directory:

- **lib.conf**: library settings
- **input.txt**: search queries and/or URLs
- **yt-dlp.conf**: yt-dlp options

Note that **input.txt** and **yt-dlp.conf** are only needed if you intend to use the library for downloading.

Removing the `tapeworm/LIBRARY` folder is all that is needed to remove the library from tapeworm's control. **Caution:** if you also downloaded files here, you might not want to delete those.

#### lib.conf

This specifies library settings, in newline-separated `name=value` pairs. If this file is not present, these defaults listed below are used. The **Command** column indicates which command(s) use that setting.

| Setting | Default | Command | Description |
|:-|:-|:-|:-|
| CLEAR_INPUT | false | `download` | Clear input.txt after downloading |
| DEPOSIT_AZ | false | `deposit` | If `TARGET_DIR` is set, enabling this will make it move files into alphabetic subdirectories of the target folder, instead of immediately in the target folder. See the example below. |
| DESCRIPTION | | `show` | Description of the library, used for informational purposes |
| INPUT_DIR | | `tag`, `deposit` | The folder where the `tag` and `deposit` commands take their inputs from. If you use the `download` command, you'll generally want yt-dlp to put its downloads into this folder, so they can be processed further. The folder is either a LIBRARY-relative path or an absolute path. **Required** for `tag` and `deposit` commands. |
| OVERRIDE_ARTIST | false | `tag` | For some sites, such as YouTube, yt-dlp will set the 'artist' tag to the uploader instead of the actual artist (which might not be available in the metadata). If the artist can be parsed from the title, setting this option will allow it to override the (incorrect) artist set by the metadata. Other sites, such as bandcamp and soundcloud, do have the correct 'artist' metadata. This is intended to be used for downloading music from YouTube, where the uploader is not the artist per se. |
| STEPS | | `process` | A comma-separated list of commands (`process` and `add` excluded). This is a convenience option, see the music library example |
| TARGET_DIR | | `deposit` | Files are downloaded according to the settings in `yt-dlp.conf`. Set this option to move files to the target folder, **after all processing** is done (e.g. downloading and tagging). Only files are moved, not directories. Files will be overwritten if already present in the target folder. TARGET_DIR expects either a path relative to the library config directory or an absolute path. **Requires** `INPUT_DIR` to be set. |
| VERBOSE | false | any | Show verbose output |

How `DEPOSIT_AZ` works:

```
# DEPOSIT_AZ=false (default)
TARGET_DIR/hello.mp3
TARGET_DIR/world.mp3
TARGET_DIR/Artist - Painting.jpg
TARGET_DIR/Band - Song.mp3

# DEPOSIT_AZ=true
TARGET_DIR/A/Artist/Artist - Painting.jpg
TARGET_DIR/B/Band/Band - Song.mp3
TARGET_DIR/H/hello.mp3
TARGET_DIR/W/world.mp3
```

#### input.txt

When adding a URL or query with `tapeworm add LIBRARY URL`, it is appended to this file if not already present. The file is created if it did not exist yet.
Each line is treated as a separate URL or query. A query may consist of one or more terms. Empty lines or lines prefixed by `#` are ignored.

An example:
```
the artist - a song
https://youtube.com/watch?v=123
```

#### yt-dlp.conf

This specifies download options for yt-dlp, see [yt-dlp](https://github.com/yt-dlp/yt-dlp) for valid options. tapeworm invokes yt-dlp as follows:

```
# If yt-dlp.conf is present:
yt-dlp --config-location ~/.config/tapeworm/LIBRARY/yt-dlp.conf [URL...]

# If yt-dlp.conf is not present:
yt-dlp [URL...]

# [URL...] is read from LIBRARY/input.txt
```

Note that files are downloaded to the directory where `tapeworm` was invoked, *unless* yt-dlp.conf specifies differently in e.g. the `-P` or `-o` option.

Also note that if you want to use the tagging/depositing feature, the `INPUT_DIR` in `lib.conf` should match the path where yt-dlp downloads to.

### Examples

Setup a library for downloading songs:
```sh
mkdir ~/.config/tapeworm/song
cd ~/.config/tapeworm/song
echo "CLEAR_INPUT=true" > lib.conf # empty input.txt when done
echo "-x <etc>" > yt-dlp.conf # add audio extraction and format options

# Add to song/input.txt
tapeworm add song https://youtube.com/watch?v=123
tapeworm add song the artist - a song

# Find URLs for each input (if needed) and download all of them as audio
tapeworm download song
```

Setup music library with tagging. The Music folder only contains properly processed (tagged) files, and `LIBRARY/tmp` is used as temporary storage for downloads.
```sh
mkdir ~/.config/tapeworm/music
cd ~/.config/tapeworm/music
echo "CLEAR_INPUT=true" >> lib.conf # empty input.txt when done
echo "INPUT_DIR=tmp" >> lib.conf
echo "TARGET_DIR=/home/<user_name>/Music" >> lib.conf

tapeworm add music https://youtube.com/watch?v=123
tapeworm add music the artist - a song

tapeworm download music
tapeworm tag music
tapeworm deposit music # to move them into TARGET_DIR

# Alternative for the above 3 commands
echo "STEPS=download,tag,deposit" >> lib.conf
tapeworm process music
```
For tagging to work, the following yt-dlp.conf setup is required:
```
# If needed, modify your metadata with --parse-metadata or --replace-metadata
# Required: embed the metadata. The title is set by default - you can modify it, but make sure it is set to something if you actually want the tagger to do something
--embed-metadata

# Add your other options, e.g. extraction and format, etc
-x
-P '~/.config/tapeworm/music/tmp'
...
```

Setup a library for archiving youtube channels:
```sh
mkdir ~/.config/tapeworm/mychannels
cd ~/.config/tapeworm/mychannels
touch archive.txt
echo "<your config options> -o '~/Videos/%(channel)/%(title)s.%(ext)s' --download-archive archive.txt" > yt-dlp.conf
echo "https://www.youtube.com/c/MyChannel/videos" > input.txt
echo "https://www.youtube.com/c/MyGamingChannel/videos" >> input.txt
# Note that we don't clear the input, as we are reusing it
# to periodically archive videos from these exact channels

tapeworm download mychannels # call this every once in a while
```

You can also use tapeworm without the downloading/tagging features, and exploit its filesystem organization capabilities. See this example for a library of pictures, where each picture's filename has to be formatted like `ARTIST - TITLE`, the artist can of course also be a source or event:
```sh
mkdir pics
echo "STEPS=deposit" >> lib.conf
echo "INPUT_DIR=tmp" >> lib.conf # dir to put images in
echo "TARGET_DIR=/home/USER/Pictures" >> lib.conf # dir to sort images into
echo "DEPOSIT_AZ=true" >> lib.conf

# Put some picture files in the folder
mv ~/Downloads/artist-painting.jpg tmp
mv ~/Downloads/dog.jpg tmp
mv "~/Downloads/France holiday 2024 - beach.png" tmp

# Sort the pictures into the target directory
tapeworm process pics
# Pictures are now in:
# Pictures/A/artist/artist-painting.jpg
# Pictures/D/dog.jpg
# Pictures/F/France holiday 2024/France holiday 2024 - beach.png
```

## Tagging

The tagging feature exploits the information often contained in an uploaded video title, for example: `The Band ft. Artist - A Song (2000) [Instrumental]`. In order for this to work, make sure your `yt-dlp.conf` is set up with metadata options. The tagger uses the `title` metadata, so at least that field should be set (setting `--embed-metadata` is enough for this). See the music library example under [Examples](#examples).

Note that if you have metadata options in `yt-dlp.conf` these are always applied, enabling or disabling tagging does not change that. Tagging only acts as an additional processing step.

## Supported websites for scraping

The following websites can currently be scraped:

- Spotify playlists

