use clap::Parser;
use dtar::archive;
use dtar::cli::{Args, Commands};

type Result<T> = anyhow::Result<T>;

fn main() -> Result<()> {
    let args = Args::parse();
    let result = match args.cmd {
        Commands::Create {
            archive,
            directories,
            create_dedup_map,
            verbose,
            follow_symlinks: _,
        } => archive::create_archive(archive, directories, create_dedup_map, verbose),
    };

    Ok(result?)
}
