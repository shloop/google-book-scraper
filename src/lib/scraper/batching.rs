use std::io::Read;
use std::io::{self};

use ::scraper::{Html, Selector};

use super::helpers::*;
use super::types::*;
use super::*;

/// Downloads all issues within the selected period of the page at the provided URL.
pub fn download_period(url: &str, dest: &str, options: &mut ScraperOptions) -> io::Result<()> {
    for issue_url in get_issue_urls_in_period(url)? {
        if let Err(x) = download_issue(&issue_url, dest, options) {
            eprintln!("Error downloading issue {issue_url}: {}", x);
        }
    }
    Ok(())
}

/// Downloads all issues within the series of the issue at the provided URL.
pub fn download_all(url: &str, dest: &str, options: &mut ScraperOptions) -> io::Result<()> {
    for period_url in get_period_urls(url)? {
        if let Err(x) = download_period(&period_url, dest, options) {
            eprintln!("Error downloading period {period_url}: {}", x);
        }
    }
    Ok(())
}

/// Gets the URLs of available periods in the page at the provided URL.
pub fn get_period_urls(url: &str) -> io::Result<Vec<String>> {
    let mut ret = Vec::new();

    let mut res = reqwest::blocking::get(url).to_result()?;
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let doc = Html::parse_document(&body);

    let selector = Selector::parse("#period_selector a").to_result()?;
    for element in doc.select(&selector) {
        if let Some(x) = element.attr("href") {
            ret.push(if x.trim() == "" {
                url.to_string()
            } else {
                x.to_string()
            });
        }
    }

    if ret.is_empty() {
        ret.push(url.to_string());
    }

    Ok(ret)
}

/// Gets the URLs of issues within the selected period of the page at the provided URL.
pub fn get_issue_urls_in_period(url: &str) -> io::Result<Vec<String>> {
    let mut ret = Vec::new();

    let mut res = reqwest::blocking::get(url).to_result()?;
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let doc = Html::parse_document(&body);

    let selector = Selector::parse("div.allissues_gallerycell a:first-child").to_result()?;
    for element in doc.select(&selector) {
        if let Some(x) = element.attr("href") {
            ret.push(x.to_string());
        }
    }

    Ok(ret)
}
