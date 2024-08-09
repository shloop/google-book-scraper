# google-book-scraper

### Purpose

This is a tool for downloading the contents of magazine archives hosted by Google Books for offline viewing. It supports conversion to PDF and/or CBZ, with the former retaining Google's provided table of contents as a document outline when available.

I had wanted to create a personal archive of one of the publications they host, but I wasn't satisifed with the features available in any of the existing scrapers I could find so I created my own.

### Disclaimers

This tool is not intended to break copyright laws and is for personal use only. It merely automates the retrieval of publicly available data using the same API calls that are used when viewing the data in a browser. The copyright of the data retrieved belongs to its repective owners and I am not responsible for any illegal redistribution of data retrieved by this tool.

Use of this tool is at your own risk.

### Basic Setup and Usage

A Windows x64 release is provided in the [releases](https://github.com/shloop/google-book-scraper/releases) section. If you are not on Windows and/or wish to build from source and have a [Rust development environment](https://www.rust-lang.org/tools/install) set up, you can run:

```
git clone https://github.com/shloop/google-book-scraper
cd google-book-scraper
cargo build --release
```

(The compiled executable will be located in *google-book-scraper/target/release*.)

To download a single issue of a magazine as a PDF to the current directory, provide the URL from an issue's *About* page. If you are on the reader page, click *About this magazine* in the left column to get to the *About* page.

```
gbscraper <URL>
```

### Batch Downloads

You can use the download mode option (`-m` or `--download-mode`) with a value of `period` to download all issues in the selected period of the URL (generally a full year or range of years), or a value of `full` to download every issue available. If specifying `full`, the provided URL can be the *About* page of any issue of the magazine.

When downloading a lot of issues it is recommended to use the archive option (`-a` or `--archive`) to keep track of which issues have already been downloaded so that they can be skipped if the operation is interrupted and needs to be restarted later.

For example:

```
gbscraper -m full -a archive.txt <URL>
```

### All Options
```
Usage: gbscraper [OPTIONS] <URL>

Arguments:
  <URL>  URL of book to download

Options:
  -o, --target-dir <DIRECTORY>  Directory to save issue(s) to [default: .]
  -k, --keep-images             If set, downloaded images will not be deleted after conversion
  -f, --format <FORMAT>...      Format(s) to convert downloaded images to [default: pdf] [possible values: none, pdf, cbz, all]
  -m, --download-mode <MODE>    Which issues to download from URL [default: single] [possible values: single, period, full]
  -a, --archive <ARCHIVE>       Don't include books in provided file. File will be updated with books downloaded
  -h, --help                    Print help
  -V, --version                 Print version
```