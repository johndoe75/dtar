use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use walkdir::DirEntry;

pub type FileMap = HashMap<String, Vec<FileInfo>>;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub dir_entry: DirEntry,
    // pub path: String,
    pub hash: Vec<u8>,
    pub size: u64,
}

impl FileInfo {
    pub fn new(direntry: DirEntry) -> Result<Self, std::io::Error> {
        let size = direntry.metadata()?.len();
        Ok(Self {
            dir_entry: direntry,
            hash: vec![],
            size,
        })
    }

    pub fn set_hash(&mut self, hash: Vec<u8>) {
        self.hash = hash;
    }

    pub fn hash_to_hex(&self) -> String {
        self.hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn path_as_string(&self) -> String {
        self.dir_entry.path().to_str().unwrap().to_string()
    }

    // On absolute paths we need to remove the leading slash to get a relative path
    pub fn sanitize_path(&self) -> String {
        self.path_as_string().trim_start_matches('/').to_string()
    }

    // Check if this is an empty file (0 bytes)
    pub fn is_empty(&self) -> bool {
        self.size == 0
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

pub struct Archive {
    pub orig_size: u64,
    pub dedup_size: u64,
}

impl Archive {
    pub fn new() -> Self {
        Self {
            orig_size: 0,
            dedup_size: 0,
        }
    }

    pub fn add_file(&mut self, file: &FileInfo) {
        self.orig_size += file.size;
        self.dedup_size += file.size;
    }

    pub fn add_dup(&mut self, file: &FileInfo) {
        self.orig_size += file.size;
    }
    
    pub fn get_orig_size(&self) -> u64 {
        self.orig_size
    }
    
    pub fn get_dedup_size(&self) -> u64 {
        self.dedup_size
    }
    
    pub fn get_dedup_ratio(&self) -> f64 {
        self.dedup_size as f64 / self.orig_size as f64
    }
}
