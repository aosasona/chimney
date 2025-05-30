use std::{fs::File, path::PathBuf};

pub mod local;

pub trait Filesystem {
    /// List the contents of a directory.
    fn read_dir(self, path: PathBuf) -> Vec<File>;

    /// Read a file from the filesystem.
    fn read_file(self, path: PathBuf) -> Option<File>;
}
