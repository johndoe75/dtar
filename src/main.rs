use clap::Parser;
use dtar::cli::{Args, Commands};
use dtar::map::{DedupMap, FileInfo, FileMap};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::{Builder, EntryType, Header};
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

    eprintln!("Collecting files from {}", dir);
    let all_files = collect_files(dir)?;

    // Parallelize the hash map generation:
    // - map: calculate the hash for each file
    // - fold: each thread generates its own local hash map for a chunk of files
    // - reduce: combines all thread-local hash maps into one final map
    eprintln!("Generating hashes for deduplication");
    let map = all_files
        .par_iter()
        .map(calc_file_hash)
        .flatten()
        .fold(HashMap::new, |mut acc: FileMap, file| {
            acc.entry(file.hash_to_hex()).or_default().push(file);
            acc
        })
        .reduce(HashMap::new, |mut map1, map2| {
            for (hash, mut files) in map2 {
                map1.entry(hash).or_default().append(&mut files);
            }
            map1
        });

    eprintln!("Writing the archive to {}", archive);
    let tar_file = File::create(archive)?;
    let mut builder = Builder::new(tar_file);
    let mut dedup_map = DedupMap::new();

    // We cannot parallelize the tar writing! This needs to be done sequentially.
    for (hash, files) in map {
        eprintln!("a {}", files[0].sanitize_path());

        // Duplicates -- write the first file as a file, create hard links for the rest
        if files.len() > 1 {
            let primary = &files[0];
            builder.append_path_with_name(&primary.path, &primary.sanitize_path())?;
            dedup_map.add_file(&hash, PathBuf::from(&files[0].path));

            for dup in &files[1..] {
                let mut header = Header::new_gnu();
                header.set_entry_type(EntryType::Link);
                header.set_size(0);

                builder.append_link(&mut header, &dup.sanitize_path(), &primary.path)?;
                dedup_map.add_file(&hash, PathBuf::from(&dup.path));
            }
        } else {
            // dedup_map.add_file(&hash, PathBuf::from(&files[0].path));
            builder.append_path_with_name(&files[0].path, &files[0].sanitize_path())?;
        }
    }

    builder.finish()?;
    dedup_map.save("foobar.json")?;

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
