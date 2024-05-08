# tapeworm

tapeworm is a scraper and downloader written in Rust. It uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) and can download whatever yt-dlp can download. tapeworm is not just a wrapper for yt-dlp, but sets itself apart with the features:

- Scrape websites for URLs or queries, see [supported websites](#supported-websites-for-scraping)
- Download (scraped) URLs and queries
- Manage different yt-dlp configurations

## Is this for you?

If you just need to download URL(s), use yt-dlp. yt-dlp has options for specifying an input file and configuration files. yt-dlp also works with queries like `yt-dlp ytsearch:"query"`. If that is not enough and you need some of the following features, tapeworm is for you:

- You want to obtain URLs/queries from sites not supported by yt-dlp, e.g. yt-dlp cannot download from Spotify; but tapeworm can scrape Spotify for song information and will download the songs using `ytsearch` queries
- You want a single application to both store URLs and for downloading them
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

tapeworm works with "libraries". A library is a URL/query collection managed by tapeworm. For example, this is the minimum setup and usage:

```sh
# Create the library by recording the first query
tapeworm add LIBRARY the artist - a song # records "the artist - a song"
# Add a URL
tapeworm add LIBRARY https://youtube.com/watch?v=123
# Scrape/download all
tapeworm download LIBRARY
```

If you add a URL from a [scraping supported site](#supported-websites-for-scraping), tapeworm will scrape that page to find song information and add that as a `ytsearch` query to the library.

Downloading the library will first download each input (whether URL or query), and may then process the downloaded files further, e.g. tagging audio files.

The behavior of the `download` command and subsequent processing is determined by the library configuration.

### Configuration

The config directory shall refer to one of the following paths (depending on your system):

- Unix: `/home/USER/.config/tapeworm/LIBRARY/`
- Windows: `/c/Users/USER/AppData/Roaming/tapeworm/LIBRARY/`

tapeworm will try to find the following files in this directory:

- **lib.conf**: library settings
- **input.txt**: search queries and/or URLs
- **yt-dlp.conf**: yt-dlp options

Removing the `tapeworm/LIBRARY` folder is all that is needed to remove the library. **Caution:** if you also downloaded files here, you might not want to delete those.

#### lib.conf

This specifies library settings, in newline-separated `name=value` pairs. If this file is not present, these defaults are used:

| Setting name | Default value | Description |
|:-|:-|:-|
| CLEAR_INPUT | false | Clear input.txt after downloading |
| DEPOSIT_AZ | false | If `TARGET_DIR` is set, enabling this will make it move files into alphabetic subdirectories of the target folder, instead of immediately in the target folder. See the example below. |
| ENABLE_TAGGING | false | Tag downloaded files. **Requires** `YT_DLP_OUTPUT_DIR` to be set. |
| TARGET_DIR | | Files are downloaded according to the settings in `yt-dlp.conf`. Set this option to move files to the target folder, **after all processing** is done (e.g. downloading and tagging). Only files are moved, not directories. Files will be overwritten if already present in the target folder. TARGET_DIR expects either a path relative to the library config directory or an absolute path. **Requires** `YT_DLP_OUTPUT_DIR` to be set. |
| VERBOSE | false | Show verbose output |
| YT_DLP_OUTPUT_DIR | | The folder where yt-dlp puts its downloads. Either a LIBRARY-relative path or an absolute path. Any file in this folder will be tagged, and possibly moved to `TARGET_DIR`. **Required** for `ENABLE_TAGGING` and `TARGET_DIR`. |

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

Also note that if you want to use the tagging feature, the `YT_DLP_OUTPUT_DIR` in `lib.conf` should match the path where yt-dlp downloads to.

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
echo "ENABLE_TAGGING=true" >> lib.conf
echo "YT_DLP_OUTPUT_DIR=tmp" >> lib.conf
echo "TARGET_DIR=/home/<user_name>/Music" >> lib.conf

tapeworm add music https://youtube.com/watch?v=123
tapeworm add music the artist - a song

# Find URLs, download, and tag
tapeworm download music
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

## Tagging

The tagging feature exploits the information often contained in an uploaded video title, for example: `The Band ft. Artist - A Song (2000) [Instrumental]`. In order for this to work, make sure your yt-dlp.conf is set up with metadata options. The tagger uses the `title` metadata, so at least that field should be set. See the music library example under [Examples](#examples).

## Supported websites for scraping

The following websites can currently be scraped:

- Spotify playlist

