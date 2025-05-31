use std::path::PathBuf;

use super::Filesystem;

pub struct LocalFS {
    path: PathBuf,
}

impl LocalFS {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Filesystem for LocalFS {
    fn read_dir(self, path: PathBuf) -> Vec<std::fs::File> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file) = std::fs::File::open(entry.path()) {
                    files.push(file);
                }
            }
        }
        files
    }

    fn list_files(self, path: PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                files.push(entry.path());
            }
        }
        files
    }

    fn read_file(self, path: PathBuf) -> Result<std::fs::File, crate::error::ChimneyError> {
        std::fs::File::open(path).map_err(crate::error::ChimneyError::IOError)
    }
}
