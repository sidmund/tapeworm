use crate::types;
use crate::util;
use crate::Config;

// When auto_scrape is enabled, the first found URL will be returned
pub fn scrape_page(
    config: &Config,
    tab: &headless_chrome::Tab,
    page: String,
) -> types::StringOptionResult {
    tab.navigate_to(page.as_str())?;

    let mut results: Vec<(String, String)> = Vec::new();
    for result_html in tab.wait_for_elements(".title-and-badge")? {
        let attributes = result_html
            .wait_for_element("a")?
            .get_attributes()?
            .unwrap();

        if config.verbose {
            println!("Found attributes: {}", attributes.join(" "));
        }

        let title = attributes.get(7).unwrap().clone();
        let rel_url = attributes.get(9).unwrap(); // /watch?v=VIDEO_ID&OTHER_ARGS
        let url = format!(
            "https://www.youtube.com{}",
            rel_url.split("&").next().unwrap()
        );

        results.push((title, url));
        if config.auto_scrape {
            break;
        }
    }

    if results.is_empty() {
        println!("No results found for '{}', skipping", page);
        return Ok(None);
    }

    if config.auto_scrape {
        let url = results.get(0).unwrap().1.clone();
        println!("Found: {}", url);
        return Ok(Some(url));
    }

    let selected = loop {
        println!("Select a result:");
        results.iter().enumerate().for_each(|(i, (title, url))| {
            println!("  {}. {} | {}", i + 1, title, url);
        });
        let index = util::input()?.parse::<usize>();
        if index.as_ref().is_ok_and(|i| *i > 0 && *i <= results.len()) {
            break index.unwrap() - 1;
        }
        println!("Invalid input, please try again");
    };
    Ok(Some(results.get(selected).unwrap().1.clone()))
}
