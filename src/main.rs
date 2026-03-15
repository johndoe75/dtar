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

        Commands::Extract {
            verbose,
            extract_to,
            archive,
        } => archive::extract_archive(extract_to, archive, verbose),
    };

    Ok(result?)
}
