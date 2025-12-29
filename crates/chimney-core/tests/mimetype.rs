use chimney::server::mimetype::{DEFAULT_MIME_TYPE, from_extension, from_path};
use std::path::PathBuf;

#[test]
fn test_from_extension() {
    assert_eq!(from_extension("txt"), "text/plain");
    assert_eq!(from_extension("html"), "text/html");
    assert_eq!(from_extension("jpg"), "image/jpeg");
    assert_eq!(from_extension("unknown"), DEFAULT_MIME_TYPE);
}

#[test]
fn test_from_path() {
    assert_eq!(from_path(PathBuf::from("file.txt")), "text/plain");
    assert_eq!(from_path(PathBuf::from("file.html")), "text/html");
    assert_eq!(from_path(PathBuf::from("file.jpg")), "image/jpeg");
    assert_eq!(
        from_path(PathBuf::from("file.unknown")),
        DEFAULT_MIME_TYPE
    );
}
