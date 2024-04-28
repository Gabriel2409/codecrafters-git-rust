mod error;
mod git_cat_file;
mod git_hash_object;
mod git_init;
mod git_ls_tree;
mod git_object;

use std::{fs, path::PathBuf};

pub use error::{Error, Result};
use git_cat_file::git_cat_file;
use git_hash_object::git_hash_object;
use git_init::git_init;
use git_ls_tree::git_ls_tree;

use clap::{Parser, Subcommand};
use git_object::GitObject;

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
    /// Compute object ID and optionally create an object from a file - only implemented for blob
    HashObject {
        #[arg(short, help = "Actually write the object into the object database.")]
        write_obj: bool,
        #[arg(help = "Path to the file")]
        file: String,
    },
    /// List the contents of a tree object
    LsTree {
        #[arg(long, help = "Only print the name of the files")]
        name_only: bool,
        #[arg(short, help = "Recurse into sub-trees")]
        recursive: bool,
        #[arg(short, long, help = "Show object size of blob (file) entries.")]
        long: bool,
        #[arg(help = "hash corresponding to a given git <object>")]
        hash: String,
    },
}

fn main() -> Result<()> {
    let g = GitObject::from_dir("a").unwrap();
    let paths = fs::read_dir("a").unwrap();

    let mut names = paths
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect::<Vec<_>>();

    names.sort();
    for name in names {
        println!("{}", name.to_str().unwrap());
    }

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Init => git_init()?,
        // TODO: There must be a better way. Can we have incompatible args as an enum?
        Commands::CatFile {
            pretty_print,
            exit_with_zero_status_if_exists,
            type_obj,
            size,
            hash,
        } => {
            git_cat_file(
                *pretty_print,
                *exit_with_zero_status_if_exists,
                *type_obj,
                *size,
                hash,
            )?;
        }
        Commands::HashObject { write_obj, file } => git_hash_object(*write_obj, file)?,
        Commands::LsTree {
            name_only,
            recursive,
            long,
            hash,
        } => git_ls_tree(*name_only, *recursive, *long, hash)?,
    };
    Ok(())
}
