use crate::map::FileInfo;
use crate::Result;
use walkdir::WalkDir;

/// Collects information about all files within a specified directory.
///
/// This function uses a recursive directory walker to traverse through
/// the contents of the provided directory path and collects metadata for
/// each file found. The metadata is stored as a `Vec<FileInfo>` which is
/// returned as a `Result`.
///
/// # Arguments
///
/// * `dir` - A string slice that holds the path to the directory to be traversed.
///
/// # Returns
///
/// This function returns a `Result`:
/// - `Ok(Vec<FileInfo>)` containing metadata for all files in the directory.
/// - `Err(anyhow::Error)` if an error occurs during the directory traversal or file metadata extraction.
///
/// # Errors
///
/// This function will return an error if:
/// - The directory cannot be accessed or traversed.
/// - There is any failure while processing the entries (e.g., invalid file metadata).
///
/// # Dependencies
///
/// This function requires:
/// - `walkdir` crate for directory traversal.
/// - `anyhow` crate for error handling.
/// - A properly implemented `FileInfo` struct with a `new` method for metadata extraction.
pub fn collect_files(dir: &str) -> Result<Vec<FileInfo>> {
    let mut result: Vec<FileInfo> = Vec::new();

    for f in WalkDir::new(dir) {
        let entry = match f {
            Ok(entry) => entry,
            Err(err) => anyhow::bail!("Error walking directory: {}", err),
        };

        result.push(FileInfo::new(entry)?);
    }

    Ok(result)
}
