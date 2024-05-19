use crate::types;
use std::collections::HashSet;

/// Scrape a Spotify playlist for a list of songs.
/// Returns the list of songs, where each song is formatted like "TITLE ARTIST"
pub fn spotify_playlist(playlist_url: &str) -> types::HashSetResult {
    let browser = headless_chrome::Browser::default()?;
    let tab = browser.new_tab()?;
    tab.navigate_to(playlist_url)?;

    println!("Scraping {}...", playlist_url);

    let mut results = HashSet::new();

    // Attempt scraping. If any error occurs, return what's been found so far
    'outer: for _ in 0..5 {
        let elements =
            tab.wait_for_elements("div[data-testid='playlist-tracklist'] div[aria-colindex='2']");
        if elements.is_err() {
            break;
        }

        for html in elements.unwrap() {
            let text = html.get_inner_text();
            if text.is_err() {
                break;
            }

            let text = text.unwrap().replace("\n", " ");
            if text == "Title" {
                continue;
            }

            println!("Found: {}", text);
            results.insert(text);
        }

        for _ in 0..2 {
            if tab.press_key("PageDown").is_err() {
                break 'outer;
            }
        }
    }

    println!("Total unique results: {}", results.len());
    Ok(results)
}
