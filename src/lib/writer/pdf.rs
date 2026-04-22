use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Bookmark};
use lopdf::{Document, Object, Stream};
use std::collections::HashMap;
use std::{fs, io};

/// Table of contents for mapping image files to page names.
pub struct TableOfContents {
    lookup: HashMap<String, TocEntry>,
}

struct TocEntry {
    pub page_title: String,
    /// 0, 1 for italic, 2 for bold, 3 for italic bold
    pub format: u32,
    /// R,G,B
    pub color: [f32; 3],
    // TODO: descendants???
}

impl TocEntry {
    fn new(page_title: String, format: u32, color: [f32; 3]) -> TocEntry {
        TocEntry {
            page_title,
            format,
            color,
        }
    }
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self::new()
    }
}

impl TableOfContents {
    pub fn new() -> TableOfContents {
        TableOfContents {
            lookup: HashMap::<String, TocEntry>::new(),
        }
    }

    /// Adds entry to table of contents.
    ///
    /// # Arguments
    ///
    /// * `page_title` - Title of page as it will appear in document outline.
    /// * `page_filename` - Filename of image to link to.
    pub fn add_page(&mut self, page_title: &str, page_filename: &str) {
        self.add_page_internal(
            page_filename,
            TocEntry::new(page_title.to_string(), 0, [0., 0., 0.]),
        );
    }

    /// Adds entry to table of contents.
    ///
    /// # Arguments
    ///
    /// * `page_title` - Title of page as it will appear in document outline.
    /// * `page_filename` - Filename of image to link to.
    /// * `format` - 0, 1 for italic, 2 for bold, 3 for italic bold.
    /// * `color` - R,G,B
    pub fn add_page_extra(
        &mut self,
        page_title: &str,
        page_filename: &str,
        format: u32,
        color: [f32; 3],
    ) {
        self.add_page_internal(
            page_filename,
            TocEntry::new(page_title.to_string(), format, color),
        );
    }

    fn add_page_internal(&mut self, page_filename: &str, entry: TocEntry) {
        self.lookup.insert(page_filename.to_string(), entry);
    }

    fn get_page_info(&self, page_filename: &String) -> Option<&TocEntry> {
        self.lookup.get(page_filename)
    }
}

/// Creates a PDF from images in a specified directory.
///
/// # Arguments
///
/// * `image_dir` - Directory where images to be converted into pages of PDF exist.
/// * `target_filename` - Path to save PDF to, including filename and extension.
pub fn create_pdf(image_dir: &str, target_filename: &str) -> io::Result<()> {
    create_pdf_internal(image_dir, target_filename, None)
}

/// Creates a PDF from images in a specified directory.
///
/// # Arguments
///
/// * `image_dir` - Directory where images to be converted into pages of PDF exist.
/// * `target_filename` - Path to save PDF to, including filename and extension.
/// * `toc` - Table fo contents mapping image files to page titles.
pub fn create_pdf_with_toc(
    image_dir: &str,
    target_filename: &str,
    toc: &TableOfContents,
) -> io::Result<()> {
    create_pdf_internal(image_dir, target_filename, Some(toc))
}

fn create_pdf_internal(
    image_dir: &str,
    target_filename: &str,
    toc: Option<&TableOfContents>,
) -> io::Result<()> {
    // Initialize document
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    // Add page for each image
    let mut pages = vec![];
    let mut entries: Vec<_> = fs::read_dir(image_dir)?
        .collect::<io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for p in entries {
        let name = p.file_name().into_string().map_err(|file_name| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("image filename is not valid UTF-8: {:?}", file_name),
            )
        })?;

        let image_path = p.path();
        let image_path_str = image_path.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("image path is not valid UTF-8: {:?}", image_path),
            )
        })?;

        let stream = lopdf::xobject::image(image_path_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to load image '{}': {e}", name),
            )
        })?;
        let content = Content {
            operations: Vec::<Operation>::new(),
        };
        let encoded_content = content.encode().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to encode PDF content stream for {}: {e}", name),
            )
        })?;
        let content_id = doc.add_object(Stream::new(dictionary! {}, encoded_content));

        let mut width: i64 = 800;
        let mut height: i64 = 1100;
        if let Ok(Object::Integer(a)) = stream.dict.get("Width".as_bytes()) {
            width = *a;
        }
        if let Ok(Object::Integer(a)) = stream.dict.get("Height".as_bytes()) {
            height = *a;
        }

        let image_filename = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        });

        doc.insert_image(
            image_filename,
            stream,
            (0., 0.),
            (width as f32, height as f32),
        )
        .map_err(|err| {
            io::Error::other(format!("failed to insert image '{name}' into PDF: {err}"))
        })?;

        pages.push(image_filename.into());

        // Check for TOC entry for this page
        if let Some(t) = toc {
            if let Some(value) = t.get_page_info(&name) {
                let b = Bookmark::new(
                    value.page_title.clone(),
                    value.color,
                    value.format,
                    image_filename,
                );
                doc.add_bookmark(b, None);
            }
        }

        //TODO: links in page
        //Note: may need to download image without setting "w=3000" first in order to scale coordinates
    }

    // Finalize and save document
    let len = pages.len() as u32;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => pages,
            "Count" => len,
            "Resources" => resources_id,
        }),
    );
    let outline_id = doc.build_outline();
    if let Some(ol) = outline_id {
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
            "Outlines" => ol,
        });
        doc.trailer.set("Root", catalog_id);
    } else {
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
    }

    doc.compress();
    doc.save(target_filename)?;
    Ok(())
}
