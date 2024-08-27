use bitflags::bitflags;
use scraper::selectable::Selectable;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{self};

use super::helpers::*;

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
}

impl Default for ScraperOptions {
    fn default() -> Self {
        Self {
            keep_images: false,
            formats: FormatFlags::Pdf,
            already_downloaded: HashSet::new(),
            archive_file: None,
            skip_download: false,
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

#[derive(Serialize, Deserialize)]
pub struct PageJson {
    pub pid: String,
    pub src: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct IssueJson {
    pub page: Vec<PageJson>,
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
    /// Gets the shortest title identifying this book.
    pub fn get_title(&self) -> &str {
        match self.book_type {
            ContentType::Magazine | ContentType::Newspaper => &self.publish_date,
            ContentType::Book => &self.series_name,
        }
    }

    /// Gets the full title of this book, including the series name if it is a magazine issue.
    pub fn get_full_title(&self) -> String {
        match self.book_type {
            ContentType::Magazine | ContentType::Newspaper => {
                std::format!("{} - {}", &self.series_name, &self.publish_date)
            }
            ContentType::Book => self.series_name.to_string(),
        }
    }

    fn parse_length(text: &str) -> io::Result<u32> {
        Ok(text
            .replace(" pages", "")
            .trim()
            .parse::<u32>()
            .to_result()?)
    }

    /// Extracts metadata from webpage.
    pub fn from_page(id: &str, doc: &Html) -> io::Result<BookMetadata> {
        let element = doc
            .select(&Selector::parse("#summary_content_table").to_result()?)
            .next()
            .to_result("Metadata could not be parsed.")?;

        let series_name = match element
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

        if let Some(e) = element
            .select(&Selector::parse("#metadata").to_result()?)
            .next()
        {
            // TODO: improve parsing here for when fields are missing
            let mut i: u32 = 0;
            for child in e.text() {
                match i {
                    0 => {
                        publish_date = child.to_string();
                    }
                    1 => {
                        length = Self::parse_length(child)?;
                    }
                    2 => {
                        volume = child.to_string();
                    }
                    3 => {
                        issn = child.to_string();
                    }
                    4 => {
                        publisher = child.to_string();
                    }
                    _ => (),
                }
                i += 1;
            }
        };

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
                        // "Title" => {
                        //     series_name = value.to_string();
                        // }
                        "Author" => {
                            author = value.to_string();
                        }
                        "Publisher" => {
                            publisher = value.to_string();
                        }
                        "Original from" => {
                            orig_from = value.to_string();
                        }
                        "Digitized" => {
                            date_digitized = value.to_string();
                        }
                        "Length" => {
                            length = Self::parse_length(value)?;
                        }
                        _ => (),
                    }
                }
            }
        }

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
            series_name,
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
