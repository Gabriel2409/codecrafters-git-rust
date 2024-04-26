mod constants;
mod error;
mod git_cat_file;
mod git_init;
mod zlib_decompress;

pub use error::{Error, Result};
use git_cat_file::git_cat_file;
use git_init::git_init;
use std::env;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about="Custom git", long_about=None )]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an empty Git repository.
    Init,
    ///  Provide content or type and size information for repository objects
    CatFile {
        #[arg(
            short,
            help = "Pretty-print the contents of <object> based on its type.",
            group = "only_accept_hash", // args part of the same group are incompatible
            conflicts_with = "allow_unkown_type" // did not find a better way. 
            // TODO: Investigate how to make two arg groups incompatible
        )]
        pretty_print: bool,

        #[arg(
            short,
            help = "Exit with zero status if <object> exists and is a valid object",
            group = "only_accept_hash",
            conflicts_with = "allow_unkown_type" // did not find a better way. 
        )]
        exit_with_zero_status_if_exists: bool,

        #[arg(
            short,
            help = "Instead of the content, show the object type identified by <object>.",
            group = "allow_unkown_type", // not implemented but follows more closely real git cat-file
            conflicts_with = "only_accept_hash" // did not find a better way. 
        )]
        type_obj: bool,

        #[arg(short, help = "Instead of the content, show the <object> size",
            group = "allow_unkown_type",
            conflicts_with = "only_accept_hash" // did not find a better way. 

        )]
        size: bool,

        #[arg(help = "hash corresponding to a given git <object>")]
        hash: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Init => git_init()?,
        Commands::CatFile { .. } => git_cat_file(&vec!["".to_string()])?,
    };
    Ok(())
}
