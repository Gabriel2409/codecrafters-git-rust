mod error;
mod git_cat_file;
mod git_init;

pub use error::{Error, Result};
use git_cat_file::git_cat_file;
use git_init::git_init;
use std::env;

fn main() -> Result<()> {
    // Uncomment this block to pass the first stage
    let args: Vec<String> = env::args().collect();

    match args[1].as_str() {
        "init" => {
            git_init(&args)?;
        }
        "cat-file" => git_cat_file(&args),
        _ => eprintln!("unknown command: {}", args[1]),
    }
    Ok(())
}
