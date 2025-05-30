use std::path::PathBuf;

use super::Filesystem;

pub struct LocalFS {
    path: PathBuf,
}

impl Filesystem for LocalFS {
    fn read_dir(self, path: PathBuf) -> Vec<std::fs::File> {
        todo!()
    }

    fn read_file(self, path: PathBuf) -> Option<std::fs::File> {
        todo!()
    }
}
