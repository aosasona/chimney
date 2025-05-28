use std::{fs::File, path::PathBuf};

pub mod local;

trait Filesystem {
    fn read_dir(self, path: PathBuf) -> Vec<File>;
}
