use crate::hasher::calc_file_hash;
use crate::Result;
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
    pub fn new(direntry: DirEntry) -> Result<Self> {
        let size = direntry.metadata()?.len();
        Ok(Self {
            dir_entry: direntry,
            hash: vec![],
            size,
        })
    }

    /// Sets the hash for the current object.
    pub fn set_hash(mut self, hash: Vec<u8>) -> Result<Self> {
        self.hash = hash;
        Ok(self)
    }

    pub fn hash_to_hex(&self) -> String {
        self.hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn path_as_string(&self) -> String {
        self.dir_entry.path().to_str().unwrap().to_string()
    }

    pub fn sanitize_path(&self) -> String {
        self.path_as_string().trim_start_matches('/').to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Calculate the sha256 hash for the file
    pub fn with_calculated_hash(mut self) -> Result<Self> {
        let mut file = File::open(self.dir_entry.path())?;
        let hash = calc_file_hash(&mut file)?;

        self.hash = hash;
        Ok(self)
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
        if self.orig_size == 0 {
            return 0.0;
        }
        self.dedup_size as f64 / self.orig_size as f64
    }
}

#[cfg(test)]
mod tests {
    use super::FileInfo;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use walkdir::{DirEntry, WalkDir};

    fn make_file_info(hash: Vec<u8>) -> (FileInfo, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let dir = std::env::temp_dir().join(format!("dtar_hash_to_hex_test_{unique}"));
        fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("dummy.bin");
        fs::write(&file_path, b"test").unwrap();

        let dir_entry: DirEntry = WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        (
            FileInfo {
                dir_entry,
                hash,
                size: 0,
            },
            dir,
        )
    }

    #[test]
    fn hash_to_hex_returns_empty_string_for_empty_hash() {
        let (file_info, dir) = make_file_info(vec![]);

        assert_eq!(file_info.hash_to_hex(), "");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hash_to_hex_converts_single_zero_byte() {
        let (file_info, dir) = make_file_info(vec![0x00]);

        assert_eq!(file_info.hash_to_hex(), "00");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hash_to_hex_preserves_leading_zeroes() {
        let (file_info, dir) = make_file_info(vec![0x00, 0x01, 0x0a, 0x0f]);

        assert_eq!(file_info.hash_to_hex(), "00010a0f");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hash_to_hex_converts_known_sequence() {
        let (file_info, dir) = make_file_info(vec![0xde, 0xad, 0xbe, 0xef]);

        assert_eq!(file_info.hash_to_hex(), "deadbeef");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hash_to_hex_uses_lowercase_hex_letters() {
        let (file_info, dir) = make_file_info(vec![0xab, 0xcd, 0xef]);

        assert_eq!(file_info.hash_to_hex(), "abcdef");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hash_to_hex_handles_all_possible_byte_values() {
        let hash: Vec<u8> = (0u8..=255).collect();
        let (file_info, dir) = make_file_info(hash);

        let hex = file_info.hash_to_hex();

        assert_eq!(hex.len(), 512);
        assert!(hex.starts_with("00010203"));
        assert!(hex.ends_with("fcfdfeff"));

        fs::remove_dir_all(dir).unwrap();
    }
}
