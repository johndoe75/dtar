use crate::map::{FileInfo, FileMap};
use sha2::digest::Update;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;

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

pub fn insert_into_file_map(mut acc: FileMap, file: FileInfo) -> FileMap {
    let key = if file.is_empty() {
        format!("empty: {}", file.path_as_string())
    } else {
        file.hash_to_hex()
    };

    acc.entry(key).or_default().push(file);
    acc
}
