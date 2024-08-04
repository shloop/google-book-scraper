use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Bookmark};
use lopdf::{Document, Object, Stream};
use std::collections::HashMap;
use std::{fs, io};

struct TocEntry {
    pub title: String,
    pub description: String,
    pub page_id: String,
}

impl TocEntry {
    pub fn new(title: String, description: String, page_id: String) -> TocEntry {
        TocEntry {
            title,
            description,
            page_id,
        }
    }
}

fn create_pdf(image_dir: &str, dest: &str, toc: &HashMap<String, TocEntry>) -> io::Result<()> {
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
    let paths = fs::read_dir(image_dir)?;
    for path in paths {
        if let Ok(p) = path {
            let name = p.file_name().into_string().unwrap();

            if let Ok(stream) = lopdf::xobject::image(p.path().as_os_str().to_str().unwrap()) {
                let content = Content {
                    operations: Vec::<Operation>::new(),
                };
                let content_id =
                    doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

                let mut width: i64 = 800;
                let mut height: i64 = 1100;
                if let Object::Integer(a) = stream.dict.get("Width".as_bytes()).unwrap() {
                    width = *a;
                }
                if let Object::Integer(a) = stream.dict.get("Height".as_bytes()).unwrap() {
                    height = *a;
                }

                let page_id = doc.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "Contents" => content_id,
                    "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
                });

                let result =
                    doc.insert_image(page_id, stream, (0., 0.), (width as f32, height as f32));
                if result.is_err() {
                    println!("error!: {name}")
                }

                pages.push(page_id.into());

                if let Some(value) = toc.get(&name) {
                    let b = Bookmark::new(value.title.clone(), [0.0, 0.0, 0.0], 0, page_id);
                    doc.add_bookmark(b, None);
                }
            }
        }
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
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
        "Outlines" => outline_id.unwrap(),
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(dest)?;
    Ok(())
}

fn main() {
    println!("Starting...");

    let mut dict = HashMap::<String, TocEntry>::new();
    dict.insert(
        "a2.png".to_string(),
        TocEntry::new("A2".to_string(), "".to_string(), "".to_string()),
    );
    dict.insert(
        "a.png".to_string(),
        TocEntry::new("A1".to_string(), "".to_string(), "".to_string()),
    );

    match create_pdf("images", "issue.pdf", &dict) {
        Ok(_) => {
            println!("Finished writing...");
        }
        Err(e) => {
            println!("Failed... {}", e)
        }
    }
}
