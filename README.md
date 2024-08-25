# google-book-scraper

### Purpose

This is a tool for downloading material hosted by Google Books for offline viewing. It was designed for the purpose of batch downloading their magazine archives, but should work for any publicly available book and will attempt to download the available preview pages of any book not publicly available. It supports conversion to PDF and/or CBZ, with the former retaining Google's provided table of contents as a document outline when available.

There are other similar tools out there, but the ones I could find didn't have the features I needed and weren't written in a language I enjoyed working in so I made my own.

### Disclaimers

This tool is not intended to break copyright laws and is for personal use only. It merely automates the retrieval of publicly available data using the same API calls that are used when viewing the data in a browser. The copyright of the data retrieved belongs to its respective owners and I am not responsible for any illegal redistribution of data retrieved by this tool.

Use of this tool is at your own risk.

### Installation

A portable Windows x64 release is provided in the [releases](https://github.com/shloop/google-book-scraper/releases) section. Alternatively, if you and have a [Rust development environment](https://www.rust-lang.org/tools/install) set up, you can install with Cargo by simply running:

```
cargo install google-book-scraper
```

### Basic Usage

To download a single book or issue of a magazine as a PDF to the current directory, provide the its URL as a command line argument.

```
gbscraper <URL>
```

### Batch Downloads

If downloading a magazine, you can use the download mode option (`-m` or `--download-mode`) with a value of `period` to download all issues in the selected period of the URL (generally a full year or range of years), or a value of `full` to download every issue available. If specifying `full`, the provided URL can be the *About* page of any issue of the magazine.

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