use scraper::{ElementRef, Html, Selector};
use std::io;
use std::io::Read;
use url::Url;
// use crate::pdf::TableOfContents;

struct IssueMetadata {
    id: String,
    series_name: String,
    publish_date: String,
    volume: String,
    issn: String,
    publisher: String,
    description: String,
}

impl IssueMetadata {
    fn new(id: &str) -> IssueMetadata {
        IssueMetadata {
            id: id.to_string(),
            series_name: String::new(),
            publish_date: String::new(),
            volume: String::new(),
            issn: String::new(),
            publisher: String::new(),
            description: String::new(),
        }
    }

    fn parse(&mut self, element: &ElementRef) {
        let mut i: u32 = 0;
        for child in element.text() {
            println!("{0}: {1}", i, child);

            match i {
                0 => {
                    self.publish_date = child.to_string();
                }
                2 => {
                    self.volume = child.to_string();
                }
                3 => {
                    self.issn = child.to_string();
                }
                4 => {
                    self.publisher = child.to_string();
                }
                _ => (),
            }
            i += 1;
        }
    }
}

pub fn download_issue(url: &str, dest: &str) -> io::Result<()> {
    println!("Scraping {url} into {dest}...");

    let mut url_obj = Url::try_from(url).unwrap();

    let mut id = String::new();
    for (key, val) in url_obj.query_pairs() {
        if key == "id" {
            id = val.to_string();
            break;
        }
    }

    let mut res = reqwest::blocking::get(url).unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let doc = Html::parse_document(&body);

    let mut issue_meta = IssueMetadata::new(&id);

    let selector = Selector::parse("#metadata").unwrap();

    for element in doc.select(&selector) {
        issue_meta.parse(&element);
        break;
    }
    let json_query_str = std::format!("id={id}&lpg=1&pg=1&jscmd=click3");
    url_obj.set_query(Some(&json_query_str));

    // TODO
    // fetch json
    // make lookup of all pages referenced in json
    // make lookup of page numbers by id, number pages in order that do not have src property
    // while lookup is not empty:
    //   pop first item and use id to fetch json
    //   download images for each page with src in json and remove ids from lookup

    Ok(())
}

pub fn download_period(url: &str, dest: &str) -> io::Result<()> {
    for issue_url in get_issue_urls_in_period(url)? {
        download_issue(&issue_url, dest)?;
    }
    Ok(())
}

pub fn download_all(url: &str, dest: &str) -> io::Result<()> {
    for period_url in get_period_urls(url)? {
        download_period(&period_url, dest)?;
    }
    Ok(())
}

pub fn get_period_urls(url: &str) -> io::Result<Vec<String>> {
    let mut ret = Vec::new();

    let mut res = reqwest::blocking::get(url).unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let doc = Html::parse_document(&body);

    let selector = Selector::parse("#period_selector a").unwrap();
    for element in doc.select(&selector) {
        if let Some(x) = element.attr("href") {
            ret.push(if x.trim() == "" {
                url.to_string()
            } else {
                x.to_string()
            });
        }
    }

    Ok(ret)
}

pub fn get_issue_urls_in_period(url: &str) -> io::Result<Vec<String>> {
    let mut ret = Vec::new();

    let mut res = reqwest::blocking::get(url).unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let doc = Html::parse_document(&body);

    let selector = Selector::parse("div.allissues_gallerycell a:first-child").unwrap();
    for element in doc.select(&selector) {
        if let Some(x) = element.attr("href") {
            ret.push(x.to_string());
        }
    }

    Ok(ret)
}
