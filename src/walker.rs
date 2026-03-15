use walkdir::WalkDir;
use crate::map::FileInfo;
use crate::Result;

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