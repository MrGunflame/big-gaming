use std::io::Read;
use std::path::Path;

use super::items::Item;

/// Loader for a json file.
pub struct JsonLoader<'a> {
    path: &'a Path,
}

impl<'a> JsonLoader<'a> {
    pub fn new<P>(path: P) -> Vec<Item>
    where
        P: AsRef<Path>,
    {
        let mut file = std::fs::File::open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        serde_json::from_slice(&buf).unwrap()
    }
}
