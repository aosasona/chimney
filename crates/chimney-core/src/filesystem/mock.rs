use super::{AbstractFile, Filesystem};

// The various files and directories that the mock filesystem will use and their contents.
static MOCK_FILES: &[(&str, &str)] = &[
    (
        "index.html",
        "<html><body><h1>Welcome to Chimney!</h1></body></html>",
    ),
    (
        "public/style.css",
        "body { font-family: Arial, sans-serif; }",
    ),
    ("public/script.js", "console.log('Hello, Chimney!');"),
    (
        "about.html",
        "<html><body><h1>About Chimney</h1></body></html>",
    ),
    (
        "contact.html",
        "<html><body><h1>Contact Us</h1></body></html>",
    ),
    (
        "404.html",
        "<html><body><h1>Page Not Found</h1></body></html>",
    ),
    (
        "data/example.json",
        r#"{"name": "Chimney", "version": "1.0.0"}"#,
    ),
    ("data/note.txt", "This is a note for Chimney."),
];

/// A mock filesystem implementation for testing purposes.
#[derive(Debug, Clone, Default)]
pub struct MockFilesystem;

impl Filesystem for MockFilesystem {
    fn read_dir(
        &self,
        path: std::path::PathBuf,
    ) -> Result<Vec<AbstractFile>, super::FilesystemError> {
        let dirname = path.to_string_lossy();
        let mut files = Vec::new();

        for (file_name, _) in MOCK_FILES {
            if file_name.starts_with(dirname.as_ref()) {
                let file_path = path.join(file_name);
                files.push(AbstractFile::new(file_path, super::FileType::File));
            }
            // if path_str.ends_with(file_name) || path_str == *file_name {
            //     let file_path = path.join(file_name);
            //     files.push(AbstractFile::new(file_path, super::FileType::File));
            // }
        }

        Ok(files)
    }

    fn list_files(
        &self,
        path: std::path::PathBuf,
    ) -> Result<Vec<std::path::PathBuf>, super::FilesystemError> {
        let dirname = path.to_string_lossy();
        let files: Vec<std::path::PathBuf> = MOCK_FILES
            .iter()
            .filter_map(|(file_name, _)| {
                if file_name.starts_with(dirname.as_ref()) {
                    Some(path.join(file_name))
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }

    fn read_file(
        &self,
        path: std::path::PathBuf,
    ) -> Result<super::Content, super::FilesystemError> {
        let path_str = path.to_string_lossy();
        for (file_name, content) in MOCK_FILES {
            if path_str.ends_with(file_name) || path_str == *file_name {
                return Ok(super::Content::new(content.to_string()));
            }
        }

        Err(super::FilesystemError::ReadFileError {
            path,
            message: "File not found in mock filesystem".to_string(),
        })
    }

    fn get_file_metadata(
        &self,
        path: std::path::PathBuf,
    ) -> Result<AbstractFile, super::FilesystemError> {
        let path_str = path.to_string_lossy();
        for (file_name, _) in MOCK_FILES {
            if path_str == *file_name {
                return Ok(AbstractFile::new(path, super::FileType::File));
            }
        }

        Err(super::FilesystemError::MetadataError {
            path,
            message: "File not found in mock filesystem".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_filesystem_read_dir() {
        let fs = MockFilesystem;
        let path = std::path::PathBuf::from("public");
        let files = fs.read_dir(path).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("style.css")));
        assert!(files.iter().any(|f| f.path.ends_with("script.js")));
    }

    #[test]
    fn test_mock_filesystem_list_files() {
        let fs = MockFilesystem;
        let path = std::path::PathBuf::from("data");
        let files = fs.list_files(path).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("example.json")));
        assert!(files.iter().any(|f| f.ends_with("note.txt")));
    }

    #[test]
    fn test_mock_filesystem_read_file() {
        let fs = MockFilesystem;
        let path = std::path::PathBuf::from("index.html");
        let content = fs.read_file(path).unwrap();

        assert_eq!(
            content.text(),
            "<html><body><h1>Welcome to Chimney!</h1></body></html>"
        );
    }

    #[test]
    fn test_mock_filesystem_get_file_metadata() {
        let fs = MockFilesystem;
        let path = std::path::PathBuf::from("about.html");
        let file = fs.get_file_metadata(path).unwrap();

        assert!(file.is_file());
        assert_eq!(file.path.to_string_lossy(), "about.html");
    }

    #[test]
    fn test_mock_filesystem_file_not_found() {
        let fs = MockFilesystem;
        let path = std::path::PathBuf::from("nonexistent.txt");
        let result = fs.read_file(path);
        assert!(result.is_err());
        if let Err(crate::filesystem::FilesystemError::ReadFileError { path, message }) = result {
            assert_eq!(path.to_string_lossy(), "nonexistent.txt");
            assert_eq!(message, "File not found in mock filesystem");
        } else {
            panic!("Expected ReadFileError");
        }
    }

    #[test]
    fn test_mock_filesystem_metadata_not_found() {
        let fs = MockFilesystem;
        let path = std::path::PathBuf::from("nonexistent.txt");
        let result = fs.get_file_metadata(path);
        assert!(result.is_err());
        if let Err(crate::filesystem::FilesystemError::MetadataError { path, message }) = result {
            assert_eq!(path.to_string_lossy(), "nonexistent.txt");
            assert_eq!(message, "File not found in mock filesystem");
        } else {
            panic!("Expected MetadataError");
        }
    }
}
