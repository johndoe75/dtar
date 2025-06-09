use duptar::cli::{Args, Commands};
use duptar::map::{FileInfo, FileMap};
use sha2::{Digest, Sha256};
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

    let mut map = FileMap::new();

    for f in WalkDir::new(dir) {
        let entry = f?;
        let path = entry
            .path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        let hash = Sha256::digest(path.as_bytes());

        let file_info = FileInfo {
            path: path.to_string(),
            hash: hash.to_vec(),
            size: entry.metadata()?.len(),
        };

        map.entry(file_info.hash_to_hex())
            .or_insert_with(Vec::new)
            .push(file_info);
    }

    println!("{:#?}", map);

    Ok(())
}
