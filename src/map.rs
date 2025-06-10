use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

pub type FileMap = HashMap<String, Vec<FileInfo>>;

#[derive(Debug)]
pub struct FileInfo {
    pub path: String,
    pub hash: Vec<u8>,
    pub size: u64,
}

impl FileInfo {
    pub fn hash_to_hex(&self) -> String {
        self.hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    // On absolute paths we need to remove the leading slash to get a relative path
    pub fn sanitize_path(&self) -> String {
        self.path.trim_start_matches('/').to_string()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DedupMap {
    files: HashMap<String, (PathBuf, Vec<PathBuf>)>,
}

impl DedupMap {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, hash: &String, path: PathBuf) {
        match self.files.entry(hash.parse().unwrap()) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert((path, Vec::new()));
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                e.get_mut().1.push(path);
            }
        }
    }

    /// Get the original files from the hash map which are stored as a file in the tar
    pub fn get_originals(&self, hash: &str) -> Option<&PathBuf> {
        self.files.get(hash).map(|(original, _)| original)
    }

    /// Get the duplicate files from the hash map which are stored as a hard link in the tar
    pub fn get_duplicates(&self, hash: &str) -> Option<&Vec<PathBuf>> {
        self.files.get(hash).map(|(_, dups)| dups)
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }
}
