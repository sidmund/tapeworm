# tapeworm

tapeworm is a media file processing utility written in Rust.

## :sparkles: Features

- Store different sets of URLs and/or queries in separate media libraries
- Scrape websites for URLs or queries
- Download (scraped) URLs and queries using library-specific settings
- Uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) to download
- Extract (additional) tags from the `title` tag for [supported extensions](https://docs.rs/audiotags/latest/audiotags/#supported-formats)
- Organize files into a media library

## :hammer: Build

A Rust installation is required. tapeworm compiles with Rust 1.74.0+ (stable).

To build tapeworm:
```bash
git clone https://github.com/sidmund/tapeworm
cd tapeworm
cargo build --release
./target/release/tapeworm help
```

## :rocket: Usage

tapeworm provides various independent "building blocks" (commands) that a **library** may use to configure its functionality. A **library**, as a dedicated media collection and manager, tends to specify *what* to download, *how* to download, and how to *process* downloads. Some examples of what you might set up (also see [detailed examples including configuration](#bulb-examples)):

- "YouTube channel archiver": a library setup that only uses the `download` functionality
- "Music downloader": a library that uses both `download` and `tag` modules
- "File organizer": only uses `deposit` functionality

Any folder can be converted into a tapeworm library by creating the `.tapeworm` subfolder. For example, to make the music folder a tapeworm library:
```bash
cd ~/Music
mkdir .tapeworm
```

The `.tapeworm` folder may contain the following files:

- **input.txt**: search queries and/or URLs (only needed for `add` and `download`)
- **lib.conf**: library settings, see [configuration](#wrench-configuration)
- **yt-dlp.conf**: yt-dlp options (only needed for `download`)

How these files are used by different commands is explained below.

### :information_source: Running commands

tapeworm is run in one of the following ways:
```bash
# a) From inside the library folder
cd ~/Library  # contains .tapeworm subfolder and the library folders/files
tapeworm COMMAND [OPTIONS]  # note that the library does not need to be specified

# b) From anywhere using the library path
tapeworm "~/Library" COMMAND [OPTIONS]

# c) From anywhere using an alias
tapeworm "~/Library" alias lib
tapeworm lib COMMAND [OPTIONS]
```

The `alias` command supports the following operations:
```bash
# List all aliases and the paths they point to
tapeworm list

# Create / Overwrite an alias
tapeworm "~/Ipsum" alias lorem

# Show the path the alias points to
tapeworm lorem alias

# Delete an alias
tapeworm lorem alias -r
```

Aliases cannot be named the same as any tapeworm command. The aliased directory must point to an existing folder with a `.tapeworm` subfolder inside it. Aliases are stored as newline-separated `alias=path` pairs in the general configuration file located at:

- Unix: `/home/<USER>/.config/tapeworm/tapeworm.conf/`
- Windows: `/c/Users/<USER>/AppData/Roaming/tapeworm/tapeworm.conf/`

In the following sections `LIBRARY` can be either a library path, an alias, or be omitted if inside the library folder.

### :link: Storing URLs and queries

The `add` command stores URLs and queries into a library's `input.txt` file. These are then used as inputs for the [download](#-downloading) command. The `input.txt` file is created if not present. Each line in this file is treated as a separate URL or query. Note that non-URLs are added as YouTube search queries.
```bash
tapeworm LIBRARY add song  # add a query, creates the input.txt file
tapeworm LIBRARY add "the artist - a song"  # add a query
tapeworm LIBRARY add https://youtube.com/watch?v=123  # add a URL
tapeworm LIBRARY add https://youtube.com/watch?v=456 "theme song" https://youtube.com/watch?v=789
```
The library's `input.txt` now contains:
```
ytsearch:song
ytsearch:the artist - a song
https://youtube.com/watch?v=123
https://youtube.com/watch?v=456
ytsearch:theme song
https://youtube.com/watch?v=789
```

#### Supported URLs

Because `download` uses yt-dlp, [any site supported by it](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md) can be added. Since yt-dlp cannot download DRM-restricted content, tapeworm provides some workarounds for the following sites:

- Spotify playlists: song information is scraped and converted to downloadable `ytsearch` queries

### :link: Downloading

The `download` command takes *all* inputs stored in the library and processes them according to the [yt-dlp configuration](#yt-dlpconf). Inputs may be added by the `add` command, or they can be manually entered into `input.txt` inside the library folder. Note that inputs must be [supported URLs or queries](#supported-urls).
```bash
tapeworm LIBRARY download
```

#### yt-dlp.conf

This file specifies [yt-dlp options](https://github.com/yt-dlp/yt-dlp) for download, extraction, post-processing, etc. When this file is not present, the result will be the same as when invoking yt-dlp without any options (resulting in disorganized downloads).

> :warning: Files are downloaded to the directory where `tapeworm LIBRARY download` was invoked, *unless* `yt-dlp.conf` specifies differently in e.g. the `-P` or `-o` option (see [yt-dlp](https://github.com/yt-dlp/yt-dlp))

> :information_source: Tip: point `yt-dlp.conf` to download to `.tapeworm/tmp/`. `INPUT_DIR` points to that same folder by default, so you don't need to configure it for the `tag` and `deposit` commands.

### :link: Tagging

> :warning: `tag` only works on files in the `INPUT_DIR`, not files in subfolders. So `yt-dlp.conf` should not specify subfolders (of `INPUT_DIR`) in the `-P` or `-o` options, if you want it to work with this commands.

> :warning: Tagging only works with [audiotags' supported formats](https://docs.rs/audiotags/latest/audiotags/#supported-formats)

The `tag` command exploits the information often contained in an uploaded video title. For example, the title `Artist ft. Singer - Song (2024) [Instrumental]` would result in tags:
```
ARTIST=Artist;Singer
TITLE=Song [Instrumental]
YEAR=2024
```
`tag` also changes the title and filename to standardized formats, see [configuration](#wrench-configuration).

`tag` uses the `title` tag, so make sure that your library's `yt-dlp.conf` specifies metadata settings:
```
# Required: embed the metadata. This sets the title by default
--embed-metadata

# If needed, modify your (title) metadata with --parse-metadata or --replace-metadata
```
For a more worked out version, see the [music library example](#music-library-with-tagging).

> :information_source: If you have metadata options in `yt-dlp.conf` these are always applied (during `download`). Tagging only acts as an additional processing step ("extracting tags from the tags")

> :warning: If you want to use `download` and `tag` (and possibly `deposit`) together, the `INPUT_DIR` in `lib.conf` should match the path where yt-dlp outputs to, see [yt-dlp.conf](#yt-dlp.conf) and [configuration](#wrench-configuration)

### :link: Organization

> :warning: `deposit` only moves files in the `INPUT_DIR`, not folders. So `yt-dlp.conf` should not specify subfolders (of `INPUT_DIR`) in the `-P` or `-o` options, if you want it to work with this command.

The `deposit` command is meant to be used after `download` and/or `tag`, in order to organize the processed files into the library directory (or as specified by `TARGET_DIR`). There are three modes available.

#### Drop (no organization)

```bash
tapeworm LIBRARY deposit -i "path/to/downloads" -o "path/to/organize/into"
tapeworm LIBRARY deposit -i "path/to/downloads" -o "path/to/organize/into" -d DROP
```
By default, `deposit` will simply drop files into the target directory:
```
TARGET_DIR/99.mp3
TARGET_DIR/Artist - Painting.png
TARGET_DIR/hello.mp3
TARGET_DIR/painting.jpg
TARGET_DIR/Song.mp3
TARGET_DIR/Song from album.mp3
```

#### Alphabetical organization

```bash
tapeworm LIBRARY deposit -i "path/to/downloads" -o "path/to/organize/into" -d A-Z
```
This organization mode will move files to alphabetic subfolders `TARGET_DIR/A-Z/ARTIST?/ALBUM?/FILENAME.EXT` (note that `ALBUM` is not relevant for image files):
```
TARGET_DIR/0-9#/99.mp3
TARGET_DIR/A/Artist/Artist - Painting.jpg
TARGET_DIR/B/Band/Song.mp3  # has "Band" ARTIST tag
TARGET_DIR/B/Band/Album/Song from album.mp3  # has ARTIST "Band" and ALBUM "Album"
TARGET_DIR/H/hello.mp3
TARGET_DIR/P/painting.jpg
```

#### Chronological organization

```bash
tapeworm LIBRARY deposit -i "path/to/downloads" -o "path/to/organize/into" -d DATE
```
This mode will move files to dated subfolders `TARGET_DIR/YYYY/MM/FILENAME.EXT`:
```
TARGET_DIR/2024/04/painting.jpg
TARGET_DIR/2024/05/hello.mp3
...
```
This organization mode is aimed at photographs, but does of course work with any files / library.

### :chains: Cleaning

The `clean` command removes empty directories inside the library folder. Of course, the `.tapeworm` folder and its constituents are always kept.
```bash
tapeworm LIBRARY clean
```

### :chains: Processing

If a library is intended to use multiple commands in a certain order, `process` is provided to simplify the interaction with the library. Instead of manually executing each command, a list of commands can be configured. These are then executed in the specified order each time `process` is invoked.
```bash
tapeworm LIBRARY process -s download,tag
```

> :information_source: `process` only accepts the following processing steps: `download`, `tag`, `deposit`, `clean`

## :wrench: Configuration

How a library uses tapeworm's commands can be configured in the `lib.conf` file. This file specifies settings in newline-separated `name=value` pairs. If not present, the following defaults are used:

| Setting name | Default value | Applicable command | Description |
|:-|:-|:-|:-|
| AUTO_DOWNLOAD | false | `download` | By default, `download` will ask the user to confirm or delete each downloaded file. Enable this setting to automatically keep all downloads. Manual confirmation may come in useful when downloading from queries, as the results can be different than expected. |
| AUTO_OVERWRITE | false | `deposit` | By default, if a file with the same name is already present in the `TARGET_DIR`, `deposit` will ask whether to overwrite. With this option enabled, it will always overwrite |
| AUTO_TAG | false | `tag` | Write discovered tags without confirmation or possibility to edit |
| CLEAR_INPUT | false | `download` | Clear input.txt after downloading |
| DESCRIPTION | | `show` | Description of the library, used for informational purposes |
| FILENAME_TEMPLATE | `{artist} - {title}` | `tag` | Files will be formatted according to this template. See [Tag format](#tag-format). In this case, the `title` refers to the title as formatted by `TITLE_TEMPLATE`. Note that the extension should not be specified. |
| INPUT_DIR | `path/to/library/.tapeworm/tmp/`| `tag`, `deposit` | The folder where the `tag` and `deposit` commands take their inputs from. If you use the `download` command, you'll generally want yt-dlp to put its downloads into this folder, so they can be processed further. The folder is either a LIBRARY-relative path or an absolute path. **Required** for `tag` and `deposit` commands. |
| ORGANIZE | | `deposit` | By default `deposit` simply drops files straight in the target folder. With this option, files are organized per one of the modes described below. **Requires** `TARGET_DIR`. |
| OVERRIDE_ARTIST | false | `tag` | For some sites, such as YouTube, yt-dlp will set the 'artist' tag to the uploader instead of the actual artist (which might not be available in the metadata). If the artist can be parsed from the title, setting this option will allow it to override the (incorrect) artist set by the metadata. Other sites, such as bandcamp and soundcloud, do have the correct 'artist' metadata. This is intended to be used for downloading music from YouTube, where the uploader is not the artist per se. |
| STEPS | | `process` | A comma-separated list of commands (`process` and `add` excluded). This is a convenience option, see the music library example |
| TARGET_DIR | `path/to/library/` | `deposit` | Files are downloaded according to the settings in `yt-dlp.conf`, and when using `deposit` they will be organized into the library folder by default (which is most likely the behavior you want). If necessary, override this option to specify a different path. Files will be overwritten if already present in the target folder. TARGET_DIR expects either a path relative to the library config directory or an absolute path. **Requires** `INPUT_DIR` to be set. |
| TITLE_TEMPLATE | `{title} ({feat}) [{remix}]` | `tag` | The original title is formatted according to this template. See [Tag format](#tag-format). |
| VERBOSE | false | any | Show verbose output |

> :information_source: Note that the default portable behavior (keeping configuration and media files in the same library folder) can be overriden by specifying input/target directories somewhere outside the library folder.

### Tag format

A tag name must be one of the following (uppercase is allowed):

- **album**
- **album_artist**
- **artist** (main artist)
- **feat** (remaining artists)
- **genre**
- **remix**
- **title**
- **track**
- **year**

The tag name must be surrounded by `{}`. Actual tag values are substituted in, and any other characters (outside `{}`) will show up as is. If a tag has no value, the tag is omitted in its entirety. Examples:
```
# Format
{artist} - {title} ({feat}) [{year}]

# Input tags -> Output string
artist=A,title=Song,year=2024 -> "A - Song [2024]"
artist=A,title=Song           -> "A - Song"
artists=A;B;C,title=Song      -> "A - Song (B & C)
```

## :bulb: Examples

### Minimal downloading setup

Setup a library for downloading songs:
```bash
mkdir song && cd song
mkdir .tapeworm
echo "CLEAR_INPUT=true" > .tapeworm/lib.conf  # empty input.txt when done
echo "-x <etc>" > .tapeworm/yt-dlp.conf  # add audio extraction and format options

# Add to ./.tapeworm/input.txt
tapeworm add https://youtube.com/watch?v=123
tapeworm add "the artist - a song"

# Find URLs for each input (if needed) and download all of them as audio
tapeworm download
```

### Music library

tapeworm was originally conceived to be used as a YouTube music downloader, tagger, and organizer. The minimal setup for this:
```bash
cd ~/Music
mkdir .tapeworm  # convert the Music folder to a tapeworm library

tapeworm add song  # records 'ytsearch:song'
tapeworm add "the artist - a song"  # records 'ytsearch:the artist - a song'
tapeworm add https://youtube.com/watch?v=123  # records the URL

# Download all URLs/queries
tapeworm download

# Optionally, tag and deposit the downloaded files
tapeworm tag  # extract more tags from "title"
tapeworm deposit  # organize into a target folder
```
Note that each processing step is executed manually. To automate this further, setup a processing pipeline:
```bash
echo "STEPS=download,tag,deposit" >> .tapeworm/lib.conf
tapeworm process

# To fully automate each command, make sure to enable:
echo "AUTO_DOWNLOAD=true" >> .tapeworm/lib.conf
echo "AUTO_OVERWRITE=true" >> .tapeworm/lib.conf
echo "AUTO_TAG=true" >> .tapeworm/lib.conf
```

### Audio tagger

Use tapeworm exclusively to tag audio files.
```bash
cd path/to/library
echo "INPUT_DIR=path/to/files" >> .tapeworm/lib.conf
tapeworm tag
```
> :warning: If you don't setup `TARGET_DIR` and a `deposit` step, the files in `INPUT_DIR` are not moved, and `tag` will keep considering those files when invoked

### File organizer

Use tapeworm exclusively to organize a file dump into subfolders. Each time `deposit` is called, files from the `INPUT_DIR` are organized into the `TARGET_DIR`.
```bash
cd path/to/lib
echo "INPUT_DIR=path/to/files" >> .tapeworm/lib.conf
echo "TARGET_DIR=where/to/organize/files/to" >> .tapeworm/lib.conf
echo "ORGANIZE=A-Z" >> .tapeworm/lib.conf

tapeworm deposit
```

### Music library with tagging

The Music folder only contains properly processed (tagged) files, and `LIBRARY/tmp` is used as temporary storage for downloads.
```bash
cd ~/Music
mkdir .tapeworm
echo "CLEAR_INPUT=true" >> .tapeworm/lib.conf  # empty input.txt when done
# Use the default .tapeworm/tmp as input folder, and the base library folder as target folder

tapeworm add https://youtube.com/watch?v=123
tapeworm add "the artist - a song"

tapeworm download  # downloaded to .tapeworm/tmp
tapeworm tag
tapeworm deposit  # move them into ~/Music

# Alternative for the above 3 commands
echo "STEPS=download,tag,deposit" >> .tapeworm/lib.conf
tapeworm process
```
With the following `yt-dlp.conf`:
```
--embed-metadata

# Add your other options, e.g. extraction and format, etc
-x
-P '~/Music/.tapeworm/tmp'
...
```

### YouTube channel archiver

```bash
cd ~/Videos
mkdir .tapeworm
touch .tapeworm/archive.txt
echo "<your config options> -o '~/Videos/%(channel)/%(title)s.%(ext)s' --download-archive archive.txt" > .tapeworm/yt-dlp.conf
echo "https://www.youtube.com/c/MyChannel/videos" > .tapeworm/input.txt
echo "https://www.youtube.com/c/MyGamingChannel/videos" >> .tapeworm/input.txt
# Note that we don't clear the input, as we are reusing it
# to periodically archive videos from these exact channels

tapeworm download  # call this every once in a while
```

### Photograph collection

You can also use tapeworm without the downloading/tagging features, and exploit its filesystem organization capabilities. See this example for a library of pictures, where each picture's filename has to be formatted like `ARTIST - TITLE`, the artist can of course also be a source or event:
```bash
cd ~/Pictures
mkdir .tapeworm
echo "STEPS=deposit" >> .tapeworm/lib.conf
echo "ORGANIZE=A-Z" >> .tapeworm/lib.conf
# Use the default .tapeworm/tmp as input folder, and the base library folder as target folder

# Put some picture files in the input folder
mv ~/Downloads/artist-painting.jpg .tapeworm/tmp
mv ~/Downloads/dog.jpg .tapeworm/tmp
mv "~/Downloads/Holiday 2024 - beach.png" .tapeworm/tmp

# Sort the pictures into the target directory
tapeworm process
# Pictures are now in:
# Pictures/A/artist/artist-painting.jpg
# Pictures/D/dog.jpg
# Pictures/H/Holiday 2024/Holiday 2024 - beach.png
```
