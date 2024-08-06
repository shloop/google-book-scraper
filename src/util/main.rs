use google_book_downloader::*;

fn main() {
    println!("Starting...");

    page_scraper::download_issue("https://books.google.com/books?id=NlMEAAAAMBAJ", "images")
        .unwrap();

    if false {
        let mut toc = pdf::TableOfContents::new();
        toc.add_page("Page One", "page1.png");
        toc.add_page_extra("Page Two", "page2.png", 3, [255., 0., 0.]);

        match pdf::create_pdf_with_toc("images", "issue.pdf", &toc) {
            Ok(_) => {
                println!("Finished writing...");
            }
            Err(e) => {
                println!("Failed... {}", e)
            }
        }
    }
}
