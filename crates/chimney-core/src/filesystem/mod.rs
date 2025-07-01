use std::path::PathBuf;
use thiserror::Error;

use crate::error::ChimneyError;

pub mod local;
pub mod mock;

#[derive(Debug, Error)]
pub enum FilesystemError {
    #[error("{0}")]
    ChimneyError(#[from] ChimneyError),

    #[error("Failed to read directory `{path}`: {message}")]
    ReadDirError { path: PathBuf, message: String },

    #[error("Failed to read file `{path}`: {message}")]
    ReadFileError { path: PathBuf, message: String },

    #[error("Failed to list files in `{path}`: {message}")]
    ListFilesError { path: PathBuf, message: String },

    #[error("Failed to get file metadata for `{path}`: {message}")]
    MetadataError { path: PathBuf, message: String },

    #[error("File or directory `{0}` does not exist")]
    NotFound(PathBuf),

    #[error("Generic error: {0}")]
    GenericError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}

/// Represents an abstract file in the filesystem.
/// This could point to an actual file on the disk, an object store like S3, or any other storage mechanism.
/// It contains metadata about the file, such as its path, content, type, and timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbstractFile {
    pub path: PathBuf,

    /// The type of the file (e.g., file, directory, symlink).
    pub file_type: FileType,

    /// The time the file was created.
    pub created_at: Option<std::time::SystemTime>,

    /// The last time the file was modified.
    pub modified_at: Option<std::time::SystemTime>,

    /// The last time the file was accessed.
    pub accessed_at: Option<std::time::SystemTime>,

    /// The permissions of the file.
    pub permissions: Option<std::fs::Permissions>,
}

/// Represents the content of a file, including its size.
///
/// This is designed as a separate struct to encapsulate the content and its size, for lazy loading the content of file as dictated by the concrete implementation of the `Filesystem` trait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// The content of the file as a string.
    content: String,

    /// The size of the content in bytes.
    size: u64,
}

impl Content {
    /// Creates a new `Content` from a string.
    pub fn new(content: String) -> Self {
        let size = content.len() as u64;
        Self { content, size }
    }

    /// Gets the content of the file.
    pub fn text(&self) -> &str {
        &self.content
    }

    /// Gets the size of the content in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }
}

impl AbstractFile {
    /// Creates a new `AbstractFile` from a path and content.
    pub fn new(path: PathBuf, file_type: FileType) -> Self {
        Self {
            path,
            file_type,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            permissions: None,
        }
    }

    /// Gets the path of the file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns `true` if the file is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self.file_type, FileType::Directory)
    }

    /// Returns `true` if the file is a file.
    pub fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::File)
    }

    /// Returns `true` if the file is a symlink.
    pub fn is_symlink(&self) -> bool {
        matches!(self.file_type, FileType::Symlink)
    }

    /// Creates a new `AbstractFile` from a path and content, reading the file metadata.
    pub fn from_disk_path(path: PathBuf) -> Result<Self, FilesystemError> {
        let metadata = std::fs::metadata(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FilesystemError::NotFound(path.clone()),
            _ => FilesystemError::MetadataError {
                path: path.clone(),
                message: e.to_string(),
            },
        })?;

        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else if metadata.is_file() {
            FileType::File
        } else if metadata.file_type().is_symlink() {
            FileType::Symlink
        } else {
            return Err(FilesystemError::GenericError(format!(
                "Unknown file type for path: {}",
                path.display()
            )));
        };

        let created_at = metadata.created().ok();
        let modified_at = metadata.modified().ok();
        let accessed_at = metadata.accessed().ok();
        let permissions = metadata.permissions();

        Ok(Self {
            path,
            file_type,
            created_at,
            modified_at,
            accessed_at,
            permissions: Some(permissions),
        })
    }
}

pub trait Filesystem: Send + Sync {
    /// Get the list of files in a directory.
    fn read_dir(&self, path: PathBuf) -> Result<Vec<AbstractFile>, FilesystemError>;

    /// List all files in a directory.
    fn list_files(&self, path: PathBuf) -> Result<Vec<PathBuf>, FilesystemError>;

    /// Read a file's content from the filesystem.
    fn read_file(&self, path: PathBuf) -> Result<Content, FilesystemError>;

    /// Check if a file or directory exists.
    fn exists(&self, path: PathBuf) -> Result<bool, FilesystemError>;

    /// Get a file's metadata.
    fn stat(&self, path: PathBuf) -> Result<AbstractFile, FilesystemError>;
}
