use std::fs::File;
use std::io::Read;
use sha2::{Digest, Sha256};
use crate::map::{FileInfo, FileMap};

pub fn calc_file_hash(file_info: &FileInfo) -> crate::Result<FileInfo> {
    let mut file = File::open(&file_info.path_as_string())?;
    let mut hasher = Sha256::new();
    const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB
    let mut buffer = [0; BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let mut file_info_with_hash = file_info.clone();
    file_info_with_hash.set_hash(hasher.finalize().to_vec());
    Ok(file_info_with_hash)
}

/// ```rust
/// Inserts a file into the provided file map (`FileMap`) using a generated key.
///
/// # Description
/// This function inserts a `FileInfo` object into a `FileMap` (a `HashMap` where keys are strings
/// and values are vectors of `FileInfo`). The key is generated based on whether the file is empty
/// or not:
/// - If the file is empty, the key is formed as `"empty: <file_path>"`.
/// - If the file is non-empty, the key is the file's hash in hexadecimal format.
///
/// The `FileInfo` object is appended to the vector corresponding to the generated key. If the key
/// does not already exist in the map, it is created with a default vector before appending the
/// file.
///
/// # Arguments
/// * `acc` - A mutable `FileMap` representing the current mapping of keys to lists of files.
/// * `file` - A `FileInfo` object representing the file to insert into the `FileMap`.
///
/// # Returns
/// Updated `FileMap` with the new file and associated key.
///
/// # Example
/// ```
/// use std::collections::HashMap;
///
/// type FileMap = HashMap<String, Vec<FileInfo>>;
///
/// struct FileInfo {
///     path: String,
///     is_empty: bool,
/// }
///
/// impl FileInfo {
///     fn path_as_string(&self) -> String {
///         self.path.clone()
///     }
///
///     fn is_empty(&self) -> bool {
///         self.is_empty
///     }
///
///     fn hash_to_hex(&self) -> String {
///         // Generate some hash and convert to hex in the real implementation.
///         "abc123".to_string()
///     }
/// }
///
/// let mut file_map: FileMap = HashMap::new();
/// let file = FileInfo {
///     path: String::from("example.txt"),
///     is_empty: false,
/// };
///
/// file_map = insert_into_file_map(file_map, file);
/// ```
///
/// # Note
/// * This function assumes that the `FileMap` is a `HashMap` where keys are strings, and values are vectors
///   of `FileInfo`.
/// ```
pub fn insert_into_file_map(mut acc: FileMap, file: FileInfo) -> FileMap {
    let key = if file.is_empty() {
        format!("empty: {}", file.path_as_string())
    } else {
        file.hash_to_hex()
    };

    acc.entry(key).or_default().push(file);
    acc
}