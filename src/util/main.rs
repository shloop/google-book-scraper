use clap::{Parser, ValueEnum};
use gbscraper::*;
use std::collections::HashSet;

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
            already_downloaded: {
                let mut set = HashSet::<String>::new();
                if let Some(file) = self.archive.as_ref() {
                    if std::fs::exists(file)? {
                        for line in std::fs::read_to_string(file).unwrap().lines() {
                            let trimmed = line.trim();
                            if trimmed != "" {
                                set.insert(trimmed.to_string());
                            }
                        }
                    }
                }
                set
            },
            skip_download: false,
            download_attempts: self.download_attempts,
            verbose: self.verbose,
        })
    }
}

fn main() {
    let args = Args::parse();
    let mut options = args.to_options().unwrap();
    let result = match args.download_mode {
        DownloadMode::Single => {
            scraper::download_issue(&args.url, &args.target_dir, &mut options).and_then(|_| Ok(()))
        }
        DownloadMode::Period => scraper::download_period(&args.url, &args.target_dir, &mut options),
        DownloadMode::Full => scraper::download_all(&args.url, &args.target_dir, &mut options),
    };
    if let Err(x) = result {
        eprintln!("Scraper error: {}", x);
    }
}
