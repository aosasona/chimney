use std::path::PathBuf;

const DEFAULT_MIME_TYPE: &str = "application/octet-stream";

/// Returns the MIME type for a given file path.
pub fn from_path(path: PathBuf) -> &'static str {
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            return from_extension(ext_str);
        }
    }
    DEFAULT_MIME_TYPE
}

/// Returns the MIME type for a given file name.
pub fn from_filename(filename: &str) -> &'static str {
    if let Some(extension) = filename.rsplit('.').next() {
        return from_extension(extension);
    }
    DEFAULT_MIME_TYPE
}

/// Returns the MIME type for a given file path.
pub fn from_extension(extension: &str) -> &'static str {
    match extension.to_lowercase().trim_start_matches('.') {
        "aac" => "audio/aac",
        "abw" => "application/x-abiword",
        "apng" => "image/apng",
        "arc" => "application/x-freearc",
        "avif" => "image/avif",
        "avi" => "video/x-msvideo",
        "azw" => "application/vnd.amazon.ebook",
        "bmp" => "image/bmp",
        "bz" => "application/x-bzip",
        "bz2" => "application/x-bzip2",
        "cda" => "application/x-cdf",
        "csh" => "application/x-csh",
        "css" => "text/css",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "eot" => "application/vnd.ms-fontobject",
        "epub" => "application/epub+zip",
        "gz" => "application/gzip",
        "gif" => "image/gif",
        "htc" => "text/x-component",
        "htm" | "html" | "stm" => "text/html",
        "htt" => "text/webviewhtml",
        "ico" => "image/vnd.microsoft.icon",
        "ics" => "text/calendar",
        "jar" => "application/java-archive",
        "jpeg" | "jpg" => "image/jpeg",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "jsonld" => "application/ld+json",
        "jxl" => "image/jxl",
        "mid" | "midi" => "audio/midi",
        "mht" | "mhtml" | "nws" => "message/rfc822",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "mpeg" | "mpg" | "mpa" | "mpe" | "mp2" | "mpv2" => "video/mpeg",
        "mpkg" => "application/vnd.apple.installer+xml",
        "mov" | "qt" => "video/quicktime",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odt" => "application/vnd.oasis.opendocument.text",
        "oga" => "audio/ogg",
        "ogv" => "video/ogg",
        "ogx" => "application/ogg",
        "opus" => "audio/opus",
        "otf" => "font/otf",
        "png" => "image/png",
        "pdf" => "application/pdf",
        "php" => "application/x-httpd-php",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "rgb" => "image/x-rgb",
        "rar" => "application/vnd.rar",
        "rtf" => "application/rtf",
        "rtx" => "text/richtext",
        "sh" => "application/x-sh",
        "svg" => "image/svg+xml",
        "tar" => "application/x-tar",
        "tif" | "tiff" => "image/tiff",
        "ts" => "video/mp2t",
        "ttf" => "font/ttf",
        "txt" | "c" | "h" | "bas" => "text/plain",
        "vcf" => "text/vcard",
        "vsd" => "application/vnd.visio",
        "wasm" => "application/wasm",
        "wav" => "audio/wav",
        "weba" => "audio/webm",
        "webm" => "video/webm",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "xhtml" => "application/xhtml+xml",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xml" => "text/xml",
        "xul" => "application/vnd.mozilla.xul+xml",
        "zip" => "application/zip",
        "3gp" => "video/3gpp",
        "3g2" => "video/3gpp2",
        "7z" => "application/x-7z-compressed",
        _ => DEFAULT_MIME_TYPE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(from_path(PathBuf::from("file.unknown")), DEFAULT_MIME_TYPE);
    }
}
