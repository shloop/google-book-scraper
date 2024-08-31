use image::{ColorType, DynamicImage, GenericImage};
use scraper::selectable::Selectable;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::{self};
use std::io::{Read, Write};
use url::Url;

use super::helpers::*;
use super::types::*;

use crate::writer::cbz::create_cbz;
use crate::writer::pdf::{create_pdf_with_toc, TableOfContents};

/// Downloads issue at the provided URL and performs any necessary format conversion.
///
/// # Arguments
///
/// * `url` - URL of issue to download.
/// * `dest` - Filename of image to link to.
/// * `options` - Various options for how to process downloaded images.
pub fn download_issue(
    url: &str,
    dest: &str,
    options: &mut ScraperOptions,
) -> io::Result<DownloadStatus> {
    // Note: Some books have download links in page: <a class="gbmt goog-menuitem-content" id="" href="$download_url">Download $ebook_format</a>
    //       These links sometimes require captcha, so probably can't be automated.

    // TODO: ensure filename safety
    // TODO: fix TOC for books without double row indices?
    // TODO: scan for links to already downloadable books
    // TODO: add file manifests so downloads can be resumed if interrupted
    // TODO: progress bar
    // TODO: concurrent downloads? (might be a bad idea since google may flag it as unusual behavior)

    let id = id_from_url(url)?;
    let url = url_from_id(&id);

    if options.already_downloaded.contains(&id) {
        println!("Skipping already downloaded book: {id}...");
        return Ok(DownloadStatus::Skipped);
    }

    println!("Identifying book: {id}...");

    // Fetch page.
    let res = reqwest::blocking::get(url).to_result()?;
    let body = res.text().to_result()?;
    let doc = Html::parse_document(&body);

    // Parse metadata from page.
    let meta = BookMetadata::from_page(&id, &doc)?;

    // Derive paths.
    let issue_combined_id = std::format!("{0} [{1}]", meta.get_full_title(), meta.id);
    let dest = match meta.book_type {
        ContentType::Magazine | ContentType::Newspaper => {
            std::format!("{dest}/{0}", meta.title)
        }
        ContentType::Book => dest.to_string(),
    };
    let issue_pics_dir = std::format!("{dest}/{issue_combined_id}");
    let filename_pdf = std::format!("{dest}/{issue_combined_id}.pdf");
    let filename_cbz = std::format!("{dest}/{issue_combined_id}.cbz");

    println!("Found: {}", meta.get_full_title());

    // Check if image directory and any needed formats already exist on disk.

    let mut formats = options.formats.clone();
    let exists_already = std::path::Path::new(&issue_pics_dir).exists();

    if exists_already {
        if std::path::Path::new(&filename_pdf).exists() {
            formats.remove(FormatFlags::Pdf)
        }
        if std::path::Path::new(&filename_cbz).exists() {
            formats.remove(FormatFlags::Cbz)
        }

        if formats == FormatFlags::None && (exists_already || !options.keep_images) {
            println!("Already downloaded. Skipping...");
            return Ok(DownloadStatus::Skipped);
        }
    }

    // Parse TOC info.
    let mut toc_page_title_lookup: HashMap<String, String> = HashMap::<String, String>::new();
    let mut parse_msg_logged = false;
    for element in doc.select(&Selector::parse("div.toc_entry").to_result()?) {
        if !parse_msg_logged {
            println!("Parsing table of contents...");
            parse_msg_logged = true;
        }

        // Title is the text of the element.
        let mut bookmark_name = String::new();
        element.text().for_each(|x| bookmark_name += x);

        // Page ID is in link URL
        if let Some(bookmark_url) = element
            .select(&Selector::parse("a").to_result()?)
            .next()
            .and_then(|x| x.attr("href"))
        {
            if let Some(x) = Url::try_from(bookmark_url)
                .to_result()?
                .query_pairs()
                .find(|x| x.0 == "pg")
            {
                toc_page_title_lookup.insert(x.1.to_string(), bookmark_name);
            }
        }
    }

    // Fetch JSON to get info about all pages.
    let mut res = reqwest::blocking::get(get_json_url(&id, "1", "1")).to_result()?;
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

    if options.skip_download {
        return Ok(DownloadStatus::Complete(meta));
    }

    if !exists_already {
        // Create directory for saving images to.
        std::fs::create_dir_all(&issue_pics_dir)?
    }

    println!("Downloading images...");

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
        let mut res =
            reqwest::blocking::get(get_json_url(&id, &first_page, &page_id)).to_result()?;
        let mut body = String::new();
        res.read_to_string(&mut body)?;
        let issue: IssueJson = serde_json::from_str(&body).to_result()?;

        // Download images linked in JSON.
        // Note: JSON will contain an entry for every page in book. Requested page should have accompanying source URL, and adjacent pages may as well.
        for page in &issue.page {
            // Skip if already downloaded.
            if let None = &page.src {
                continue;
            }
            // Skip if no download link.
            else if pages_downloaded.contains(&page.pid) {
                continue;
            }

            let mut standard_download = true;
            let mut filename = String::new();

            let mut p = 0;
            let page_number = page_number_lookup.get(&page.pid).unwrap_or_else(|| {
                // In unlikely case where page ID was not included in original JSON, append to end of known pages.
                p = i_page;
                i_page += 1;
                &p
            });

            if let ContentType::Newspaper = meta.book_type {
                // For newspapers, only proceed if this is the requested page or high res info is present.
                if let Some(npage_info) = page
                    .additional_info
                    .as_ref()
                    .and_then(|x| x.newspaper_json_page_info.as_ref())
                {
                    // Segmented download
                    standard_download = false;

                    let size_info = npage_info
                        .tile_res
                        .last()
                        .to_result("Failed to parse newspaper size info")?;

                    let mut any_png = false;
                    let mut canvas = image::DynamicImage::new(
                        size_info.width.into(),
                        size_info.height.into(),
                        image::ColorType::Rgb8,
                    );

                    // Images are segmented into 256x256 chunks. Segments at bottom and right edges of page may be smaller.
                    const SEGMENT_MAX_W: u32 = 256;
                    const SEGMENT_MAX_H: u32 = 256;

                    // Segments are grouped into blocks of up to 3x3 for ordering.
                    const SEGMENT_GROUP_MAX_W: u32 = SEGMENT_MAX_W * 3;
                    const SEGMENT_GROUP_MAX_H: u32 = SEGMENT_MAX_H * 3;

                    // Both the groups and the segments increment left to right, top to bottom, like so:
                    //  -----------------------------
                    // | 00 01 02 | 09 10 11 | 18 19 |
                    // | 03 04 05 | 12 13 14 | 20 21 |
                    // | 06 07 08 | 15 16 17 | 22 23 |
                    // | --------- ---------- ------ |
                    // | 24 25 26 | 30 31 32 | 36 37 |
                    // | 27 28 29 | 33 34 35 | 38 39 |
                    //  -----------------------------

                    let coord_x = npage_info.page_scanjob_coordinates.x;
                    let coord_y = npage_info.page_scanjob_coordinates.y;
                    let zoom = size_info.zoom;

                    let src_url = Url::try_from(page.src.as_ref().unwrap().as_str()).to_result()?;
                    let sig = src_url
                        .query_pairs()
                        .find(|x| x.0 == "sig")
                        .to_result("msg")?
                        .1
                        .to_string();

                    let mut i = 0;
                    let mut y_group = 0;
                    while y_group < size_info.height {
                        let mut x_group = 0;
                        while x_group < size_info.width {
                            let mut y_segment = y_group;
                            while (y_segment < size_info.height)
                                && (y_segment < (y_group + SEGMENT_GROUP_MAX_H))
                            {
                                let mut x_segment = x_group;
                                while (x_segment < size_info.width)
                                    && (x_segment < (x_group + SEGMENT_GROUP_MAX_W))
                                {
                                    // TODO: retries and/or error logging.

                                    // Fetch image segment and determine format.
                                    let mut res =
                                        reqwest::blocking::get(std::format!("https://books.google.com/books/content?id={id}&pg={coord_x},{coord_y}&img=1&zoom={zoom}&hl=en&sig={sig}&tid={i}")).to_result()?;
                                    let ext = get_image_ext(&res)?;
                                    any_png |= ext == "png";

                                    // Copy segment to page image.
                                    let mut buf = vec![];
                                    _ = res.read_to_end(&mut buf).to_result()?;
                                    let other = image::load_from_memory(&buf).to_result()?;
                                    canvas.copy_from(&other, x_segment, y_segment).to_result()?;

                                    i += 1;

                                    x_segment += SEGMENT_MAX_W;
                                }
                                y_segment += SEGMENT_MAX_H;
                            }
                            x_group += SEGMENT_GROUP_MAX_W;
                        }
                        y_group += SEGMENT_GROUP_MAX_H;
                    }

                    filename = generate_image_filename(
                        page_number,
                        &page.pid,
                        if any_png { "png" } else { "jpg" },
                    );
                    canvas
                        .save(std::format!("{issue_pics_dir}/{filename}"))
                        .to_result()?;
                } else if page.pid != page_id {
                    continue;
                }
            }

            if standard_download {
                // TODO: retries and/or error logging.

                // Fetch image at highest available resolution.
                let mut res =
                    reqwest::blocking::get(std::format!("{}&w=10000", page.src.as_ref().unwrap()))
                        .to_result()?;

                // Write to disk.
                let ext = get_image_ext(&res)?;
                filename = generate_image_filename(page_number, &page.pid, &ext);

                let out_path = std::format!("{issue_pics_dir}/{filename}");

                if ext == "png" {
                    // If PNG, ensure 24bpp or else it may not appear correctly in PDF.
                    // In the future, may want to just save as is and let PDF conversion handle image conversion.
                    let mut buf = vec![];
                    _ = res.read_to_end(&mut buf).to_result()?;
                    let img = image::load_from_memory(&buf).to_result()?;
                    let img = match img.color() {
                        ColorType::Rgb8 => img,
                        _ => {
                            let mut img_24_bpp = DynamicImage::new_rgb8(img.width(), img.height());
                            img_24_bpp.copy_from(&img, 0, 0).to_result()?;
                            img_24_bpp
                        }
                    };
                    img.save(out_path).to_result()?;
                } else {
                    if let Ok(mut file) = std::fs::File::create_new(out_path) {
                        res.copy_to(&mut file).to_result()?;
                    }
                }
            }

            // If TOC entry exists for page ID, associate filename.
            if let Some(title) = toc_page_title_lookup.get(&page.pid) {
                toc.add_page(title, &filename);
            }

            pages_downloaded.insert(page.pid.clone());
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

    // Clean up downloaded images unless option is set or directory already existed.
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

    Ok(DownloadStatus::Complete(meta))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Time to wait in between requests to hopefuly not get flagged as unusual behavior.
    const WAIT_TIME: u64 = 2000;

    fn pause_between_requests() {
        std::thread::sleep(std::time::Duration::from_millis(WAIT_TIME));
    }

    #[test]
    fn metadata_parsing() {
        // Book
        {
            let id = String::from("XV8XAAAAYAAJ");
            let url = std::format!("https://books.google.com/books?id={id}");
            let dest = ".";
            let mut options = ScraperOptions::default();
            options.skip_download = true;

            let mut description = String::new();
            description.push_str("A literary classic that wasn't recognized for its merits until decades after its publication, Herman Melville's Moby-Dick");
            description.push_str(" tells the tale of a whaling ship and its crew, who are carried progressively further out to sea by the fiery Captain Ahab.");
            description.push_str(" Obsessed with killing the massive whale, which had previously bitten off Ahab's leg, the seasoned seafarer steers his ship");
            description.push_str(" to confront the creature, while the rest of the shipmates, including the young narrator, Ishmael, and the harpoon expert,");
            description.push_str(" Queequeg, must contend with their increasingly dire journey. The book invariably lands on any short list of the greatest American novels.");

            let expected = BookMetadata {
                id,
                title: String::from("Moby Dick"),
                publish_date: String::from(""),
                volume: String::from(""),
                issn: String::from(""),
                publisher: String::from("Dana Estes & Company, 1892"),
                description,
                book_type: ContentType::Book,
                author: String::from("Herman Melville"),
                length: 545,
                date_digitized: String::from("Mar 20, 2008"),
                orig_from: String::from("Harvard University"),
            };

            let metadata = download_issue(&url, dest, &mut options);
            assert_eq!(metadata.unwrap(), DownloadStatus::Complete(expected));
        }

        pause_between_requests();

        // Magazine
        {
            let id = String::from("CFEEAAAAMBAJ");
            let url = std::format!("https://books.google.com/books?id={id}");
            let dest = ".";
            let mut options = ScraperOptions::default();
            options.skip_download = true;

            let mut description = String::new();
            description.push_str("LIFE Magazine is the treasured photographic magazine that chronicled the 20th Century. It now lives on at LIFE.com,");
            description.push_str(" the largest, most amazing collection of professional photography on the internet. Users can browse, search and view");
            description.push_str(" photos of today’s people and events. They have free access to share, print and post images for personal use.");

            let expected = BookMetadata {
                id,
                title: String::from("LIFE"),
                publish_date: String::from("Oct 3, 1969"),
                volume: String::from("Vol. 67, No. 14"),
                issn: String::from("0024-3019"),
                publisher: String::from("Time Inc"),
                description,
                book_type: ContentType::Magazine,
                author: String::from(""),
                length: 94,
                date_digitized: String::from(""),
                orig_from: String::from(""),
            };

            let metadata = download_issue(&url, dest, &mut options);
            assert_eq!(metadata.unwrap(), DownloadStatus::Complete(expected));
        }

        pause_between_requests();

        //Newspaper
        {
            let id = String::from("W4clAAAAIBAJ");
            let url = std::format!("https://books.google.com/books?id={id}");
            let dest = ".";
            let mut options = ScraperOptions::default();
            options.skip_download = true;

            let expected = BookMetadata {
                id,
                title: String::from("The Afro American"),
                publish_date: String::from("Jan 4, 1992"),
                volume: String::from(""),
                issn: String::from(""),
                publisher: String::from("The Afro American"),
                description: String::from(""),
                book_type: ContentType::Newspaper,
                author: String::from(""),
                length: 0,
                date_digitized: String::from(""),
                orig_from: String::from(""),
            };

            let metadata = download_issue(&url, dest, &mut options);
            assert_eq!(metadata.unwrap(), DownloadStatus::Complete(expected));
        }
    }
}
