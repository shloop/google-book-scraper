use bitflags::bitflags;
use scraper::selectable::Selectable;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::io::{self};

use super::helpers::*;

pub use json_api::IssueJson;
pub use json_api::PageJson;

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
    /// If true, only retrieve metadata without downloading or processing images.
    pub skip_download: bool,
    /// Number of times to attempt to download any file before giving up on a book. Set to 0 to try indefinitely.
    pub download_attempts: u32,
    /// If true, extra output will be given.
    pub verbose: bool,
}

impl Default for ScraperOptions {
    fn default() -> Self {
        Self {
            keep_images: false,
            formats: FormatFlags::Pdf,
            already_downloaded: HashSet::new(),
            archive_file: None,
            skip_download: false,
            download_attempts: 3,
            verbose: false,
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

/// Metadata for book or individual issue of magazine.
#[derive(Debug, PartialEq, Eq)]
pub struct BookMetadata {
    /// ID used to identify book resource
    pub id: String,
    /// Title of book or periodical
    pub title: String,
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
    /// Type of book
    pub book_type: ContentType,
    /// Author of the book
    pub author: String,
    /// Number of pages
    pub length: u32,
    /// Date book was digitized
    pub date_digitized: String,
    /// Source of book
    pub orig_from: String,
}

/// Data types deserializing JSON API calls to get book info.
mod json_api {
    use serde::{Deserialize, Serialize};

    /// Result of API call to get metadata about a book or issue.
    #[derive(Serialize, Deserialize)]
    pub struct IssueJson {
        pub page: Vec<PageJson>,
    }

    /// Metadata pertaining to specific page.
    #[derive(Serialize, Deserialize)]
    pub struct PageJson {
        pub pid: String,
        pub src: Option<String>,
        pub additional_info: Option<PageAdditionalInfo>,
    }

    /// Additional metadata for specific page.
    #[derive(Serialize, Deserialize)]
    pub struct PageAdditionalInfo {
        #[serde(rename(deserialize = "[NewspaperJSONPageInfo]"))]
        pub newspaper_json_page_info: Option<NewspaperJsonPageInfo>,
    }

    /// Additional metadata for newspaper pages.
    #[derive(Serialize, Deserialize)]
    pub struct NewspaperJsonPageInfo {
        #[serde(rename(deserialize = "tileres"))]
        pub tile_res: Vec<TileRes>,
        pub page_scanjob_coordinates: Coordinates,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TileRes {
        #[serde(rename(deserialize = "h"))]
        pub height: u32,
        #[serde(rename(deserialize = "w"))]
        pub width: u32,
        #[serde(rename(deserialize = "z"))]
        pub zoom: u32,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Coordinates {
        pub x: u32,
        pub y: u32,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContentType {
    Book,
    Magazine,
    Newspaper,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DownloadStatus {
    Skipped,
    Complete(BookMetadata),
}

impl BookMetadata {
    const SUFFIX_PAGES: &'static str = " pages";
    const PREFIX_PUBLISHER: &'static str = "Published by ";
    const PREFIX_ISSN: &'static str = "ISSN ";

    const LABEL_TITLE: &'static str = "Title";
    const LABEL_AUTHOR: &'static str = "Author";
    const LABEL_PUBLISHER: &'static str = "Publisher";
    const LABEL_ORIG_FROM: &'static str = "Original from";
    const LABEL_DIGITIZED: &'static str = "Digitized";
    const LABEL_LENGTH: &'static str = "Length";
    const LABEL_ISBN: &'static str = "ISBN";

    /// Gets the shortest title identifying this book.
    pub fn get_title(&self) -> &str {
        match self.book_type {
            ContentType::Magazine | ContentType::Newspaper => &self.publish_date,
            ContentType::Book => &self.title,
        }
    }

    /// Gets the full title of this book, including the series name if it is a magazine issue.
    pub fn get_full_title(&self) -> String {
        match self.book_type {
            ContentType::Magazine | ContentType::Newspaper => {
                std::format!("{} - {}", &self.title, &self.publish_date)
            }
            ContentType::Book => self.title.to_string(),
        }
    }

    fn parse_length(text: &str) -> io::Result<u32> {
        Ok(Self::remove_and_extract(text, Self::SUFFIX_PAGES)
            .parse::<u32>()
            .to_result()?)
    }

    fn remove_and_extract(source: &str, to_remove: &str) -> String {
        source.replace(to_remove, "").trim().to_string()
    }

    /// Extracts metadata from webpage.
    pub fn from_page(id: &str, doc: &Html) -> io::Result<BookMetadata> {
        let element = doc
            .select(&Selector::parse("#summary_content_table").to_result()?)
            .next()
            .to_result("Metadata could not be parsed.")?;

        let mut title = match element
            .select(&Selector::parse(".booktitle").to_result()?)
            .next()
            .and_then(|e| e.text().next())
        {
            Some(x) => x.to_string(),
            _ => String::new(),
        };

        let description = match element
            .select(&Selector::parse("#synopsistext").to_result()?)
            .next()
            .and_then(|e| e.text().next())
        {
            Some(x) => x.to_string(),
            _ => String::new(),
        };

        let mut publish_date = String::new();
        let mut volume = String::new();
        let mut issn = String::new();
        let mut publisher = String::new();
        let mut author = String::new();
        let mut length = 0;
        let mut date_digitized = String::new();
        let mut orig_from = String::new();
        let mut isbn = Vec::<String>::new();

        // Main metadata area
        if let Some(e) = element
            .select(&Selector::parse("#metadata").to_result()?)
            .next()
        {
            let mut i: u32 = 0;
            for child in e.text() {
                if i == 0 {
                    publish_date = child.to_string();
                } else if child.starts_with(Self::PREFIX_PUBLISHER) {
                    publisher = Self::remove_and_extract(child, Self::PREFIX_PUBLISHER);
                } else if child.starts_with(Self::PREFIX_ISSN) {
                    issn = Self::remove_and_extract(child, Self::PREFIX_ISSN);
                } else if child.ends_with(Self::SUFFIX_PAGES) {
                    length = Self::parse_length(child)?;
                } else {
                    volume = child.to_string();
                }

                i += 1;
            }
        };

        // Bibliography area - used specifically by books?
        for tr in doc.select(&Selector::parse(".metadata_row").to_result()?) {
            if let Some(label) = tr
                .select(&Selector::parse(".metadata_label").to_result()?)
                .next()
                .and_then(|e| e.text().next())
            {
                if let Some(value) = tr
                    .select(&Selector::parse(".metadata_value span").to_result()?)
                    .next()
                    .and_then(|e| e.text().next())
                {
                    match label {
                        Self::LABEL_TITLE => {
                            title = value.to_string();
                        }
                        Self::LABEL_AUTHOR => {
                            author = value.to_string();
                        }
                        Self::LABEL_PUBLISHER => {
                            publisher = value.to_string();
                        }
                        Self::LABEL_ORIG_FROM => {
                            orig_from = value.to_string();
                        }
                        Self::LABEL_DIGITIZED => {
                            date_digitized = value.to_string();
                        }
                        Self::LABEL_ISBN => {
                            value
                                .split(",")
                                .for_each(|x| isbn.push(x.trim().to_string()));
                        }
                        Self::LABEL_LENGTH => {
                            length = Self::parse_length(value)?;
                        }
                        _ => (),
                    }
                }
            }
        }

        // Determine content type from text in preview link
        let book_type = match doc
            .select(&Selector::parse("#preview-link span").to_result()?)
            .next()
            .and_then(|e| e.text().next())
        {
            Some(x) => {
                if x.contains("magazine") {
                    ContentType::Magazine
                } else if x.contains("newspaper") {
                    ContentType::Newspaper
                } else {
                    ContentType::Book
                }
            }
            _ => ContentType::Book,
        };

        Ok(BookMetadata {
            id: id.to_string(),
            title,
            publish_date,
            volume,
            issn,
            publisher,
            description,
            book_type,
            author,
            length,
            date_digitized,
            orig_from,
        })
    }
}
