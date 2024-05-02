mod error;
mod git_cat_file;
mod git_clone;
mod git_commit_tree;
mod git_hash_object;
mod git_init;
mod git_ls_tree;
mod git_object;
mod git_pack;
mod git_write_tree;

pub use error::{Error, Result};
use git_cat_file::git_cat_file;
use git_clone::git_clone;
use git_commit_tree::git_commit_tree;
use git_hash_object::git_hash_object;
use git_init::git_init;
use git_ls_tree::git_ls_tree;
use git_write_tree::git_write_tree;

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
    /// Writes the full directory (not just what is in the staging area like in regular git)
    WriteTree,
    /// Writes the commit object based on a tree and a parent commit
    CommitTree {
        #[arg(help = "sha1 of the tree")]
        tree_sha: String,
        #[arg(short, help = "sha of parent commit")]
        parent_commit_sha: String,
        #[arg(short, help = "commit message")]
        message: String,
    },
    Clone {
        #[arg(help = "url of the repository to clone")]
        repository_url: String,
        #[arg(help = "directory to clone into")]
        directory: Option<String>,
    },
}

fn main() -> Result<()> {
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
        Commands::WriteTree => git_write_tree()?,
        Commands::CommitTree {
            tree_sha,
            parent_commit_sha,
            message,
        } => git_commit_tree(tree_sha, parent_commit_sha, message)?,
        Commands::Clone {
            repository_url,
            directory,
        } => {
            let directory = match directory {
                None => repository_url.split('/').last().ok_or(Error::Unreachable)?,
                Some(directory) => directory,
            };

            git_clone(repository_url, directory)?
        }
    };
    Ok(())
}
