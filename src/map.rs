use std::collections::HashMap;

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
