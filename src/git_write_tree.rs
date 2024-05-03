use crate::git_object::GitObject;
use crate::Result;

/// Writes the current directory to the .git folder of the current directory
/// All trees and blobs are written
pub fn git_write_tree() -> Result<()> {
    let git_obj = GitObject::from_dir(".")?;

    git_obj.write(".", true)?;
    println!("{}", git_obj.hash);

    Ok(())
}
