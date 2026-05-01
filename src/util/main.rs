use clap::{Parser, ValueEnum};
use gbscraper::*;
use std::collections::HashSet;
use crate::scraper::FALLBACK_TLD;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// URL of book to download.
    #[arg(value_name = "URL")] //(short = 'i', long, value_name = "BOOK_URL")]
    url: String,

    /// Directory to save issue(s) to.
    #[arg(
        short = 'o',
        long = "target-dir",
        value_name = "DIRECTORY",
        default_value = "."
    )]
    target_dir: String,

    /// If set, downloaded images will not be deleted after conversion.
    #[arg(short, long = "keep-images", default_value_t = false)]
    keep_images: bool,

    /// Format(s) to convert downloaded images to.
    #[arg(value_enum, short, long, value_delimiter = ',', num_args = 1.., default_value ="pdf")]
    format: Option<Vec<Format>>,

    /// Which issues to download from URL.
    #[arg(value_enum, short = 'm', long = "download-mode", value_name = "MODE", default_value_t = DownloadMode::Single)]
    download_mode: DownloadMode,

    /// Omit previously downloaded books referenced in provided file. If provided, newly downloaded books will be automatically added to file.
    #[arg(short, long)]
    archive: Option<String>,

    /// Number of times to attempt downloading any file before giving up on book. Set to 0 to try indefinitely.
    #[arg(short, long, short = 'r', default_value_t = 3)]
    download_attempts: u32,

    /// If set, extra output will be given.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// The top level domain to normalize URLs to for downloading. If omitted, ".us" will be used.
    /// Set to "none" to disable URL normalization and use TLD from provided URL.
    #[arg(short, long)]
    tld_override: Option<String>,
    
    // TODO: File naming scheme
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Format {
    None,
    Pdf,
    Cbz,
    All,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum DownloadMode {
    Single,
    Period,
    Full,
}

impl Args {
    /// Converts command line options to options for scraper methods
    fn to_options(&self) -> std::io::Result<scraper::ScraperOptions> {
        Ok(scraper::ScraperOptions {
            keep_images: self.keep_images,
            formats: {
                match self.format.as_ref() {
                    None => scraper::FormatFlags::Pdf,
                    Some(v) => {
                        let mut flags = scraper::FormatFlags::None;
                        for f in v {
                            flags |= match f {
                                Format::None => scraper::FormatFlags::None,
                                Format::Pdf => scraper::FormatFlags::Pdf,
                                Format::Cbz => scraper::FormatFlags::Cbz,
                                Format::All => scraper::FormatFlags::All,
                            }
                        }
                        flags
                    }
                }
            },
            archive_file: self.archive.clone(),
            skip_download: false,
            download_attempts: self.download_attempts,
            verbose: self.verbose,
            tld: match &self.tld_override {
                // None, provided, use default
                None => FALLBACK_TLD.to_string(),
                Some(tld) => match tld.to_lowercase().as_str() {
                    // Provided "none", disable normalization and use TLD from URL.
                    "none" => match tldextract::TldExtractor::new(tldextract::TldOption::default())
                        .extract(&self.url)
                    {
                        Ok(x) => match x.suffix {
                            Some(x) => format!(".{x}"),
                            None => FALLBACK_TLD.to_string(),
                        },
                        Err(_) => FALLBACK_TLD.to_string(),
                    },
                    // Provided TLD, use it.
                    _ => tld.to_string(),
                },
            },
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let options = match args.to_options() {
        Ok(opts) => opts,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let mut already_downloaded = HashSet::<String>::new();
    if let Some(file) = args.archive.as_ref() {
        if std::fs::exists(file)? {
            for line in std::fs::read_to_string(file)?.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    already_downloaded.insert(trimmed.to_string());
                }
            }
        }
    }
    let result = match args.download_mode {
        DownloadMode::Single => scraper::download_issue_skip_downloaded(
            &args.url,
            &args.target_dir,
            &options,
            Some(&mut already_downloaded),
        )
        .map(|_| ()),
        DownloadMode::Period => scraper::download_period(
            &args.url,
            &args.target_dir,
            &options,
            &mut already_downloaded,
        ),
        DownloadMode::Full => scraper::download_all(
            &args.url,
            &args.target_dir,
            &options,
            &mut already_downloaded,
        ),
    };
    if let Err(x) = result {
        eprintln!("Scraper error: {}", x);
    }
    Ok(())
}
