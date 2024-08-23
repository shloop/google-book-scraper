use bitflags::bitflags;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::{self};
use std::io::{Read, Write};
use url::Url;

use crate::cbz::create_cbz;
use crate::pdf::{create_pdf_with_toc, TableOfContents};

/// Scrape options.
pub struct ScraperOptions {
    /// If true, downloaded images will not be deleted after conversion.
    pub keep_images: bool,
    /// Format(s) to convert downloaded images to.
    pub formats: FormatFlags,
    /// IDs of issues to skip.
    pub already_downloaded: HashSet<String>,
    /// File to store IDs of already downloaded books.
    pub archive_file: Option<String>,
}

impl Default for ScraperOptions {
    fn default() -> Self {
        Self {
            keep_images: false,
            formats: FormatFlags::Pdf,
            already_downloaded: HashSet::new(),
            archive_file: None,
        }
    }
}

bitflags! {
    /// Format(s) downloaded images to
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FormatFlags:u32 {
        const None = 0b000;
        const Pdf =  0b001;
        const Cbz =  0b010;
        const All =  0b011;
    }
}

/// Metadata for single issue.
pub struct IssueMetadata {
    /// ID used to identify book resource
    pub id: String,
    /// Name of series/magazine issue belongs to
    pub series_name: String,
    /// Date issue was published
    pub publish_date: String,
    /// Volume of issue
    pub volume: String,
    /// ISSN of publication
    pub issn: String,
    /// Publisher
    pub publisher: String,
    /// Description of publication
    pub description: String,
}

#[derive(Serialize, Deserialize)]
struct PageJson {
    pid: String,
    src: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct IssueJson {
    page: Vec<PageJson>,
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

    /// Parses metadata from the provided HTML element.
    fn parse(&mut self, element: &ElementRef) -> io::Result<()> {
        let selector = Selector::parse(".booktitle").to_result()?;
        for e in element.select(&selector) {
            for child in e.text() {
                self.series_name = child.to_string();
                break;
            }
            break;
        }
        let selector = Selector::parse("#synopsistext").to_result()?;
        for e in element.select(&selector) {
            for child in e.text() {
                self.description = child.to_string();
                break;
            }
            break;
        }
        let selector = Selector::parse("#metadata").to_result()?;
        for e in element.select(&selector) {
            let mut i: u32 = 0;
            for child in e.text() {
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
            break;
        }
        Ok(())
    }
}

trait ToResult<T> {
    fn to_result(self) -> std::io::Result<T>;
}

impl<T, E: Display> ToResult<T> for std::result::Result<T, E> {
    fn to_result(self) -> std::io::Result<T> {
        match self {
            Ok(x) => Ok(x),
            Err(x) => Err(std::io::Error::new(io::ErrorKind::Other, x.to_string())),
        }
    }
}

/// Downloads issue at the provided URL and performs any necessary format conversion.
///
/// # Arguments
///
/// * `url` - URL of issue to download.
/// * `dest` - Filename of image to link to.
/// * `options` - Various options for how to process downloaded images.
pub fn download_issue(url: &str, dest: &str, options: &mut ScraperOptions) -> io::Result<()> {
    // TODO: get name of individual book not in series
    // TODO: support ne wstyle URLs

    // Parse ID from URL.
    let mut url_obj = Url::try_from(url).to_result()?;

    let mut id = String::new();
    for (key, val) in url_obj.query_pairs() {
        if key == "id" {
            id = val.to_string();
            break;
        }
    }

    if options.already_downloaded.contains(&id) {
        println!("Skipping already downloaded book: {id}...");
        return Ok(());
    }

    println!("Identifying book: {id}...");

    // Fetch page.
    let res = reqwest::blocking::get(url).to_result()?;
    let body = res.text().to_result()?;
    let doc = Html::parse_document(&body);

    // Parse issue metadata from page.
    let mut issue_meta = IssueMetadata::new(&id);
    let selector = Selector::parse("#summary_content_table").to_result()?;
    for element in doc.select(&selector) {
        issue_meta.parse(&element)?;
        break;
    }

    // Derive paths and create any missing directories.
    let issue_combined_id = std::format!("{0} [{1}]", issue_meta.publish_date, issue_meta.id);
    let series_dir = std::format!("{dest}/{0}", issue_meta.series_name);
    let issue_pics_dir = std::format!("{series_dir}/{issue_combined_id}");

    println!("Found: {0} - {issue_combined_id}", issue_meta.series_name);

    let exists_already = std::path::Path::new(&issue_pics_dir).exists();
    if !exists_already {
        std::fs::create_dir_all(&issue_pics_dir)?
    };

    // Check if any needed formats already exist on disk.
    let mut formats = options.formats.clone();
    let filename_pdf = std::format!("{series_dir}/{issue_combined_id}.pdf");
    if std::path::Path::new(&filename_pdf).exists() {
        formats.remove(FormatFlags::Pdf)
    }
    let filename_cbz = std::format!("{series_dir}/{issue_combined_id}.cbz");
    if std::path::Path::new(&filename_cbz).exists() {
        formats.remove(FormatFlags::Cbz)
    }

    if formats == FormatFlags::None && (exists_already || !options.keep_images) {
        println!("Already downloaded. Skipping...");
        return Ok(());
    }

    // Parse TOC info.
    let mut toc_page_title_lookup = HashMap::<String, String>::new();
    let selector = Selector::parse("#toc tr").to_result()?;
    let mut i = 0;
    let mut bookmark_text = String::new();
    let mut bookmark_page_id = String::new();
    for element in doc.select(&selector) {
        // Bookmarks encompass two <td>s, so this will alternate between title and description/keywords
        if i % 2 == 0 {
            // Title
            for span in element.select(&Selector::parse("span").to_result()?) {
                for str in span.text() {
                    bookmark_text += str;
                }
                break;
            }

            for link in element.select(&Selector::parse("a").to_result()?) {
                if let Some(href) = link.attr("href") {
                    let link_url_obj = Url::try_from(href).to_result()?;
                    for (key, val) in link_url_obj.query_pairs() {
                        if key == "pg" {
                            bookmark_page_id = val.to_string();
                            break;
                        }
                    }
                }
                break;
            }
        } else {
            // Keywords

            // Note: Ignoring this as it seems to be just keywords that make the bookmark overly long if included.
            // let mut bookmark_description = String::new();
            // for str in element.text() {
            //     bookmark_description += str;
            // }
            // while bookmark_description.contains("  ") {
            //     bookmark_description = bookmark_description.replace("  ", " ");
            // }

            toc_page_title_lookup.insert(bookmark_page_id, bookmark_text);
            bookmark_page_id = String::new();
            bookmark_text = String::new();
        }
        i += 1;
    }

    // Fetch JSON to get info about all pages.
    let json_query_str = std::format!("id={id}&lpg=1&pg=1&jscmd=click3");
    url_obj.set_query(Some(&json_query_str));
    let mut res = reqwest::blocking::get(url_obj.to_string()).to_result()?;
    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let issue: IssueJson = serde_json::from_str(&body).to_result()?;

    // Make lookup of all pages referenced in json and their absolute page number.
    let mut page_number_lookup = HashMap::<String, usize>::new();
    let mut pages_to_download = VecDeque::<String>::new();
    let mut first_page = "1".to_string();
    let mut i_page = 1;
    for page in issue.page {
        if let None = page.src {
            page_number_lookup.insert(page.pid.clone(), i_page);
            pages_to_download.push_back(page.pid.clone());
            if i_page == 1 {
                first_page = page.pid;
            }
            i_page += 1;
        }
    }

    // Download all pages and associate filenames in TOC.
    let mut toc = TableOfContents::new();
    let mut pages_downloaded = HashSet::<String>::new();
    while !pages_to_download.is_empty() {
        // Get next page ID, skip if already downloaded.
        let page_id = pages_to_download.pop_front().unwrap();
        if pages_downloaded.contains(&page_id) {
            continue;
        }

        // Fetch JSON for page.
        let json_query_str = std::format!("id={id}&lpg={first_page}&pg={page_id}&jscmd=click3");
        url_obj.set_query(Some(&json_query_str));
        let mut res = reqwest::blocking::get(url_obj.to_string()).to_result()?;
        let mut body = String::new();
        res.read_to_string(&mut body)?;
        let issue: IssueJson = serde_json::from_str(&body).to_result()?;

        // Download images linked in JSON.
        // Note: JSON will contain an entry for every page in book. Requested page should have accompanying source URL, and adjacent pages may as well.
        for page in issue.page {
            if let Some(src) = page.src {
                // Skip if already downloaded.
                if pages_downloaded.contains(&page.pid) {
                    continue;
                }

                // Fetch image at highest available resolution.
                let mut res = reqwest::blocking::get(src + "&w=10000").to_result()?;

                // Determine image type from HTTP result.
                let mut ext = "jpg";
                for (name, value) in res.headers() {
                    if name.as_str() == "content-type" {
                        ext = value.to_str().to_result()?;
                        let mut start = 0;
                        if let Some(x) = ext.find("/") {
                            start = x + 1
                        }
                        ext = &ext[start..];
                        if ext == "jpeg" {
                            ext = "jpg"
                        }
                        break;
                    }
                }

                // Generate filename based on page order, page ID, and image type.
                let mut p = 0;
                let page_number = page_number_lookup.get(&page.pid).unwrap_or_else(|| {
                    // In unlikely case where page ID was not included in original JSON, append to end of known pages.
                    p = i_page;
                    i_page += 1;
                    &p
                });
                let filename = std::format!(
                    "{0}-{1}.{2}",
                    std::format!("{:0>5}", page_number),
                    page.pid,
                    ext
                );

                // Write to disk.
                if let Ok(mut file) =
                    std::fs::File::create_new(std::format!("{issue_pics_dir}/{filename}"))
                {
                    res.copy_to(&mut file).to_result()?;
                }

                // If TOC entry exists for page ID, associate filename.
                if let Some(title) = toc_page_title_lookup.get(&page.pid) {
                    toc.add_page(title, &filename);
                }

                pages_downloaded.insert(page.pid);
            }
        }
    }

    // Download any formats not already downloaded.
    if formats.contains(FormatFlags::Pdf) {
        println!("Generating PDF...");

        create_pdf_with_toc(&issue_pics_dir, &filename_pdf, &toc)?;
    }
    if formats.contains(FormatFlags::Cbz) {
        println!("Generating CBZ...");

        create_cbz(&issue_pics_dir, &filename_cbz)?;
    }

    if !(options.keep_images || exists_already) {
        std::fs::remove_dir_all(&issue_pics_dir)?;
    }

    // All done. Add to list of downloaded books and update archive file if applicable.
    options.already_downloaded.insert(id.to_string());
    if let Some(archive) = options.archive_file.as_ref() {
        if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(archive) {
            if let Err(e) = file.write(std::format!("{id}\n").as_bytes()) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }
    }

    Ok(())
}

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
