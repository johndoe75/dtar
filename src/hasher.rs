use crate::map::{FileInfo, FileMap};
use sha2::digest::Update;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;

/// Calculates the SHA-256 hash of a file and updates the `FileInfo` object with the computed hash.
///
/// This function reads the content of the file specified in the `FileInfo` object in 1 MB chunks,
/// computes the SHA-256 hash incrementally, and sets the resulting hash value in the cloned
/// `FileInfo` object. The updated `FileInfo` object is then returned.
///
/// # Parameters
/// - `file_info`: A reference to a `FileInfo` object containing the path of the file for which the
///   hash needs to be calculated.
///
/// # Returns
/// - `Ok(FileInfo)`: A `FileInfo` object cloned from the input and updated with the computed hash
///   value.
/// - `Err(crate::Error)`: If there is an error opening the file, reading from it, or encountering
///   any other I/O-related issue.
///
/// # Errors
/// This function returns an error in the following scenarios:
/// - If the file specified in `file_info.path_as_string()` cannot be opened.
/// - If there is an error while reading the file content.
///
/// # Notes
/// - This function uses a fixed buffer size of 1 MB (`1024 * 1024`) to read the file in chunks.
/// - The `FileInfo` must implement the `set_hash` method to store the calculated hash.
/// - The function relies on the `sha2` crate for SHA-256 hashing and the `FileInfo` type to
/// manage file metadata.
pub fn calc_file_hash(file: &mut File) -> crate::Result<Vec<u8>> {
    const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB
    let mut hasher = Sha256::new();
    let mut buffer = [0; BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        Update::update(&mut hasher, &buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_vec())
}

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
/// # Note
/// * This function assumes that the `FileMap` is a `HashMap` where keys are strings, and values are vectors
///   of `FileInfo`.
pub fn insert_into_file_map(mut acc: FileMap, file: FileInfo) -> FileMap {
    let key = if file.is_empty() {
        format!("empty: {}", file.path_as_string())
    } else {
        file.hash_to_hex()
    };

    acc.entry(key).or_default().push(file);
    acc
}
