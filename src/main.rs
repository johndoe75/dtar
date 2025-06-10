use clap::Parser;
use dtar::cli::{Args, Commands};
use dtar::map::{FileInfo, FileMap};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sha2::{Digest, Sha224, Sha256};
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

    for (hash, files) in map {
        for file in files {
            println!("{} {} {}", hash, file.path, file.size);
        }

        // Duplicates
        // if files.len() > 1 {
        //     println!("Duplicate: {} {}", hash, files[0].path);
        // }
    }

    // for f in WalkDir::new(dir) {
    //     let entry = f?;
    //     let path = entry
    //         .path()
    //         .to_str()
    //         .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    //
    //     let hash = Sha256::digest(path.as_bytes());
    //
    //     let file_info = FileInfo {
    //         path: path.to_string(),
    //         hash: hash.to_vec(),
    //         size: entry.metadata()?.len(),
    //     };
    //
    //     map.entry(file_info.hash_to_hex())
    //         .or_insert_with(Vec::new)
    //         .push(file_info);
    // }
    //
    // println!("{:#?}", map);
    //
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
