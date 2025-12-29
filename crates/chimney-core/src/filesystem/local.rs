use std::path::PathBuf;

use super::{AbstractFile, Content, Filesystem, FilesystemError};

pub struct LocalFS {
    path: PathBuf,
}

impl LocalFS {
    pub fn new(path: PathBuf) -> Result<Self, FilesystemError> {
        // We will attempy to create the path if it does not exist
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| FilesystemError::GenericError(e.to_string()))?;
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Filesystem for LocalFS {
    fn read_dir(&self, path: PathBuf) -> Result<Vec<AbstractFile>, FilesystemError> {
        let files = self
            .list_files(path.clone())
            .map_err(|e| FilesystemError::ReadDirError {
                path: path.clone(),
                message: e.to_string(),
            })?;

        files
            .into_iter()
            .map(AbstractFile::from_disk_path)
            .collect()
    }

    fn list_files(&self, path: PathBuf) -> Result<Vec<PathBuf>, FilesystemError> {
        let dir = path
            .canonicalize()
            .map_err(|e| FilesystemError::ListFilesError {
                path: path.clone(),
                message: e.to_string(),
            })?;

        let entries =
            std::fs::read_dir(&dir).map_err(|e| FilesystemError::GenericError(e.to_string()))?;

        let files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();

        Ok(files)
    }

    fn read_file(&self, path: PathBuf) -> Result<Content, FilesystemError> {
        let bytes = std::fs::read(&path).map_err(|e| FilesystemError::ReadFileError {
            path: path.clone(),
            message: e.to_string(),
        })?;

        Ok(Content::new(bytes))
    }

    fn stat(&self, path: PathBuf) -> Result<AbstractFile, FilesystemError> {
        AbstractFile::from_disk_path(path)
    }

    fn exists(&self, path: PathBuf) -> Result<bool, FilesystemError> {
        let exists = path.exists();
        Ok(exists)
    }
}
