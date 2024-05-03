use std::path::Path;

use crate::git_object::GitObject;
use crate::Result;

/// Returns hash of blob.
/// Optionally writes it
pub fn git_hash_object<P: AsRef<Path>>(write_obj: bool, file: P) -> Result<()> {
    let git_obj = GitObject::from_blob(file)?;

    if write_obj {
        git_obj.write(".", true)?;
    }
    println!("{}", git_obj.hash);

    Ok(())
}
