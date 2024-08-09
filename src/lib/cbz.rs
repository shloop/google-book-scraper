use std::{
    fs,
    io::{self, Read, Seek, Write},
};
use zip::write::SimpleFileOptions;

/// Creates a CBZ from images in a specified directory.
///
/// # Arguments
///
/// * `image_dir` - Directory where images to be converted into pages of CBZ exist.
/// * `target_filename` - Path to save CBZ to, including filename and extension.
pub fn create_cbz(image_dir: &str, target_filename: &str) -> io::Result<()> {
    let dir_entry = std::path::Path::new(target_filename);
    let file = std::fs::File::create(dir_entry).unwrap();

    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let read_dir = fs::read_dir(image_dir)?;
    for dir_entry in read_dir {
        if let Ok(dir_entry) = dir_entry {
            if let Ok(mut file) = std::fs::File::open(dir_entry.path()) {
                let filename = dir_entry.file_name().into_string().unwrap();
                let _ = file.seek(io::SeekFrom::Start(0));

                zip.start_file(filename, options)?;

                let mut buffer = Vec::new();
                let _ = file.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
            }
        }
    }

    zip.finish()?;
    Ok(())
}
