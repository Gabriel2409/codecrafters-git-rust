use std::path::Path;

use crate::git_object::GitObject;
use crate::Result;

pub fn git_hash_object<P: AsRef<Path>>(write_obj: bool, file: P) -> Result<()> {
    let git_obj = GitObject::from_blob(file)?;

    // TODO: would be better with an enum
    if write_obj {
        git_obj.write(".", true)?;
    }
    println!("{}", git_obj.hash);

    Ok(())
}
