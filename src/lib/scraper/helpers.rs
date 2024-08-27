use std::fmt::Display;
use std::io::{self};
use url::Url;

/// Parse book ID from URL.
pub fn id_from_url(url: &str) -> io::Result<String> {
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
pub fn url_from_id(id: &str) -> String {
    std::format!("https://books.google.com/books?id={id}")
}

/// Gets URL of JSON pertaiing to specified page.
pub fn get_json_url(id: &str, first_page: &str, page_id: &str) -> String {
    std::format!(
        "{}&lpg={first_page}&pg={page_id}&jscmd=click3",
        url_from_id(id)
    )
}

// Methods to convert between option/result types for error propogation.

pub trait ToResult<T> {
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

pub trait ToResultErrorMessage<T> {
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
        let expected = std::format!("https://books.google.com/books?id={ID}");
        assert_eq!(url, expected);
    }
}
