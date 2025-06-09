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
    /// Create a new archive.  Required is the archive name and at least one directory or file.
    Create {
        #[clap(num_args = 1, required = true)]
        archive: String,

        #[clap(value_delimiter = ' ', num_args = 1.., required = true)]
        directories: Vec<String>,
    },
}
