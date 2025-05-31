use std::{fs::File, path::PathBuf};

use crate::error::ChimneyError;

pub mod local;

pub trait Filesystem {
    /// Get the list of files in a directory.
    fn list_files(self, path: PathBuf) -> Vec<PathBuf>;

    /// Read a directory from the filesystem.
    fn read_dir(self, path: PathBuf) -> Vec<File>;

    /// Read a file from the filesystem.
    fn read_file(self, path: PathBuf) -> Result<File, ChimneyError>;
}
