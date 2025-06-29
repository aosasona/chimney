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
        let content =
            std::fs::read_to_string(&path).map_err(|e| FilesystemError::ReadFileError {
                path: path.clone(),
                message: e.to_string(),
            })?;

        Ok(Content::new(content))
    }

    fn stat(&self, path: PathBuf) -> Result<AbstractFile, FilesystemError> {
        AbstractFile::from_disk_path(path)
    }

    fn exists(&self, path: PathBuf) -> Result<bool, FilesystemError> {
        let exists = path.exists();
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_fs_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let fs = LocalFS::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(fs.path().exists());
    }

    #[test]
    fn test_local_fs_read_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let fs = LocalFS::new(temp_dir.path().to_path_buf()).unwrap();
        fs.read_dir(temp_dir.path().to_path_buf()).unwrap();
    }

    #[test]
    fn test_local_fs_list_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let fs = LocalFS::new(temp_dir.path().to_path_buf()).unwrap();
        let files = fs.list_files(temp_dir.path().to_path_buf()).unwrap();
        assert!(files.is_empty());
    }
}
