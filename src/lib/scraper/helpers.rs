use std::fmt::Display;
use std::io::{self};
use url::Url;

/// Parse book ID from URL.
pub(crate) fn id_from_url(url: &str) -> io::Result<String> {
    // Note: old style URL: https://books.google.com/books?id=$book_id&$other_args...
    //       new style URL: https://www.google.com/books/edition/$arbitrary_title/$book_id?$args...

    let url_obj = Url::try_from(url).to_result()?;
    const INVALID_URL: &str = "Invalid URL";
    Ok(match url_obj.query_pairs().find(|x| x.0 == "id") {
        Some(x) => x.1.to_string(),
        None => url_obj
            .path_segments()
            .to_result(INVALID_URL)?
            .last()
            .to_result(INVALID_URL)?
            .to_string(),
    })
}

/// Generate basic old-style URL from book ID.
pub(crate) fn url_from_id(id: &str) -> String {
    std::format!("https://books.google.us/books?id={id}&hl=en")
}

/// Gets URL of JSON pertaiing to specified page.
pub(crate) fn get_json_url(id: &str, first_page: &str, page_id: &str) -> String {
    std::format!(
        "{}&lpg={first_page}&pg={page_id}&jscmd=click3",
        url_from_id(id)
    )
}

/// Converts URL to US/English and strips unneccessary
pub(crate) fn sanitize_url(url: &str) -> io::Result<String> {
    // Strip everything but ID and force English
    let base_url = url_from_id(&id_from_url(url)?);
    // Check for period in original URL and add to result if found
    const PERIOD_TAG: &str = "atm_aiy";
    let url_obj = Url::try_from(url).to_result()?;
    match url_obj.query_pairs().find(|x| x.0 == PERIOD_TAG) {
        Some(x) => Ok(std::format!("{base_url}&{PERIOD_TAG}={}", x.1.to_string())),
        None => Ok(base_url),
    }
}

// Methods to convert between option/result types for error propogation.

pub(crate) trait ToResult<T> {
    ///
    fn to_result(self) -> std::io::Result<T>;
}

impl<T, E: Display> ToResult<T> for std::result::Result<T, E> {
    fn to_result(self) -> std::io::Result<T> {
        match self {
            Ok(x) => Ok(x),
            Err(x) => Err(std::io::Error::new(io::ErrorKind::Other, x.to_string())),
        }
    }
}

pub(crate) trait ToResultErrorMessage<T> {
    ///
    fn to_result(self, msg: &str) -> std::io::Result<T>;
}

impl<T> ToResultErrorMessage<T> for Option<T> {
    fn to_result(self, msg: &str) -> std::io::Result<T> {
        match self {
            Some(x) => Ok(x),
            None => Err(std::io::Error::new(io::ErrorKind::Other, msg)),
        }
    }
}

/// Generate filename for image.
pub(crate) fn generate_image_filename(page_number: &usize, page_id: &str, ext: &str) -> String {
    std::format!(
        "{0}-{1}.{2}",
        std::format!("{:0>5}", page_number),
        page_id,
        ext
    )
}

/// Determine image extension by the content header.
pub(crate) fn get_image_ext(res: &reqwest::blocking::Response) -> io::Result<String> {
    let mut ext = "jpg";
    for (name, value) in res.headers() {
        if name.as_str() == "content-type" {
            ext = value.to_str().to_result()?;
            let mut start = 0;
            if let Some(x) = ext.find("/") {
                start = x + 1
            }
            ext = &ext[start..];
            if ext == "jpeg" {
                ext = "jpg"
            }
            break;
        }
    }
    Ok(ext.to_string())
}

/// Determine image extension by the content header.
pub(crate) fn try_download(url: &str, mut attempts: u32) -> io::Result<reqwest::blocking::Response> {
    let indefinite = attempts == 0;
    let mut res: io::Result<reqwest::blocking::Response> = Err(io::Error::new(io::ErrorKind::Other, ""));
    while indefinite || attempts > 0 {
        res = reqwest::blocking::get(url).to_result();
        if let Ok(res) = res {
            return Ok(res);
        }
        if !indefinite {
            attempts -= 1;
            eprintln!("Download failed for {url}. {attempts} attempt(s) remaining...");
        }
        else{
            eprintln!("Download failed for {url}. Retrying...");
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "FAKE_ID";
    const ARGS: &str = "a=aa&b=bb&c=1";

    #[test]
    fn old_url_parsing() {
        let url = std::format!("https://books.google.com/books?id={ID}&{ARGS}");
        assert_eq!(id_from_url(&url).unwrap().as_str(), ID);
    }

    #[test]
    fn new_url_parsing() {
        let url = std::format!("https://www.google.com/books/edition/_/{ID}?{ARGS}");
        assert_eq!(id_from_url(&url).unwrap().as_str(), ID);
    }

    #[test]
    fn url_fixing() {
        let url = url_from_id(ID);
        let expected = std::format!("https://books.google.us/books?id={ID}&hl=en");
        assert_eq!(url, expected);
    }
}
