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
                return Ok(super::Content::new(content.to_string().into()));
            }
        }

        Err(super::FilesystemError::ReadFileError {
            path,
            message: "File not found in mock filesystem".to_string(),
        })
    }

    fn stat(&self, path: std::path::PathBuf) -> Result<AbstractFile, super::FilesystemError> {
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

    fn exists(&self, path: std::path::PathBuf) -> Result<bool, super::FilesystemError> {
        let path_str = path.to_string_lossy();
        for (file_name, _) in MOCK_FILES {
            if path_str == *file_name || path_str.starts_with(file_name) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
