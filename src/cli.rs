use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, author, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Commands,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Create a new archive
    Create {
        #[arg(
            short,
            long,
            default_value = "false",
            help = "Create a deduplication map"
        )]
        create_dedup_map: bool,

        #[arg(
            short,
            long,
            default_value = "false",
            help = "Per default, symlinks are not followed and archived\nas symlinks. If this flag is set, symlinks are\nfollowed and archived as files."
        )]
        follow_symlinks: bool,

        #[clap(num_args = 1, required = true)]
        archive: String,

        #[clap(value_delimiter = ' ', num_args = 1.., required = true)]
        directories: Vec<String>,

        #[arg(short, long, default_value = "false", help = "Increase verbosity")]
        verbose: bool,
    },

    /// Extract an archive.
    Extract {
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Extract to a specific directory"
        )]
        extract_to: String,

        #[arg(short, long, default_value = "false", help = "Increase verbosity")]
        verbose: bool,

        #[clap(num_args = 1, required = true)]
        archive: String,
    },
}
