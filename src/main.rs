use clap::Parser;
use dtar::cli::{Args, Commands};
use dtar::map::{FileInfo, FileMap};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use tar::Builder;
use walkdir::WalkDir;

type Result<T> = anyhow::Result<T>;

fn main() -> Result<()> {
    let args = Args::parse();
    let result = match args.cmd {
        Commands::Create {
            archive,
            directories,
        } => create_archive(archive, directories),
    };

    Ok(result?)
}

fn create_archive(archive: String, directories: Vec<String>) -> Result<()> {
    let dir = directories
        .first()
        .ok_or_else(|| anyhow::anyhow!("No directories provided"))?;

    let all_files = collect_files(dir)?;

    // Eat this in parallel
    let all_files = all_files
        .par_iter()
        .map(calc_file_hash)
        .flatten()
        .collect::<Vec<FileInfo>>();

    let mut map = FileMap::new();

    for file in all_files {
        map.entry(file.hash_to_hex()).or_default().push(file);
    }

    let tar_file = File::create(archive)?;
    let mut builder = Builder::new(tar_file);

    for (_, files) in map {
        eprintln!("a {}", files[0].sanitize_path());

        // Duplicates
        if files.len() > 1 {
            let primary = &files[0];
            builder.append_path_with_name(&primary.path, &primary.sanitize_path())?;
            for dup in &files[1..] {
                let mut header = tar::Header::new_gnu();
                builder.append_link(&mut header, &dup.sanitize_path(), &primary.sanitize_path())?
            }
        } else {
            builder.append_path_with_name(&files[0].path, &files[0].sanitize_path())?;
        }
    }

    builder.finish()?;

    Ok(())
}

fn collect_files(dir: &str) -> Result<Vec<FileInfo>> {
    let mut result: Vec<FileInfo> = Vec::new();

    for f in WalkDir::new(dir) {
        let entry = f?;
        let path = entry
            .path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        result.push(FileInfo {
            path: path.to_string(),
            hash: vec![],
            size: entry.metadata()?.len(),
        });
    }

    Ok(result)
}

fn calc_file_hash(file_info: &FileInfo) -> Result<FileInfo> {
    let mut file = File::open(&file_info.path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024 * 1024]; // 1 MB buffer

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(FileInfo {
        path: file_info.path.clone(),
        hash: hasher.finalize().to_vec(),
        size: file_info.size,
    })
}
