use std::fs;

use crate::{Error, Result};

pub fn git_init(args: &[String]) -> Result<()> {
    let nb_args = args.len();
    if nb_args != 2 {
        return Err(Error::InvalidNbArgs {
            expected: 2,
            got: nb_args,
        });
    }
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}
