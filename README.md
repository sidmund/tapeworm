# tapeworm

tapeworm is a scraper and downloader written in Rust. It uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) and can download whatever yt-dlp can download. tapeworm is not just a wrapper for yt-dlp, but sets itself apart with the features:

- Scrape websites for URLs based on user-provided queries, see [supported websites](#supported-websites-for-scraping)
- Download (scraped) URLs
- Manage different yt-dlp configurations

## Is this for you?

If you just need to download URL(s), use yt-dlp. yt-dlp also has options for specifying an input file and configuration files. If that is not enough and you need some of the following features, tapeworm is for you:

- You want to have both URLs and queries as inputs, where the queries are automatically used to scrape relevant URLs
- You want a single application to both store URLs and for downloading them
- You want to setup different download options for different sets of input URLs, and be able to easily configure and invoke them. E.g. you have a music and a video library and want a single application to easily download sources for them with the right options
- You like the abstraction tapeworm provides by never having to specify the config file yourself, or worrying about what file to store URLs in, as this can all be done with simple tapeworm commands

## Build

A Rust installation is required. tapeworm compiles with Rust 1.74.0 (stable).

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
tapeworm LIBRARY the artist - a song # records "the artist - a song"
# Add a URL
tapeworm LIBRARY https://youtube.com/watch?v=123
# Scrape/download all
tapeworm LIBRARY
```

Invoking `tapeworm LIBRARY` will first scrape YouTube with each recorded search query to obtain URLs for them. Then, it will download all URLs. The behavior of this command is determined by the library configuration.

### Configuration

When using a library, tapeworm will look for the following files in `~/.config/tapeworm/LIBRARY/`:

- **lib.conf**: library settings
- **input.txt**: search queries and/or URLs
- **yt-dlp.conf**: yt-dlp options

Removing the `~/.config/tapeworm/LIBRARY` folder is all that is needed to remove the library. **Caution:** if you also downloaded files here, you might not want to delete those.

#### lib.conf

This specifies library settings. If this file is not present, the defaults are used:

```
CLEAR_INPUT=false # whether to clear input.txt after downloading
AUTO_SCRAPE=false # false to manually select a URL, true to use the first found URL
VERBOSE=false # show verbose output
```

#### input.txt

When adding a URL or query with `tapeworm LIBRARY URL`, it is appended to this file if not already present. The file is created if it did not exist yet.
Each line is treated as a separate URL or query. A query may consist of one or more terms. Empty lines or lines prefixed by `#` are ignored.

An example:
```
Europe final countdown
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

### Examples

Setup a library for downloading songs:
```sh
mkdir ~/.config/tapeworm/song
cd ~/.config/tapeworm/song
echo "CLEAR_INPUT=true" > lib.conf # empty input.txt when done
echo "-x <etc>" > yt-dlp.conf # add audio extraction and format options

# Add to song/input.txt
tapeworm song https://youtube.com/watch?v=123
tapeworm song the artist - a song

# Find URLs for each input (if needed) and download all of them as audio
tapeworm song
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

tapeworm mychannels # call this every once in a while
```

## Supported websites for scraping

The following websites can currently be scraped:

- YouTube
