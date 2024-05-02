use crate::git_object::GitObject;
use crate::Result;

pub fn git_write_tree() -> Result<()> {
    let git_obj = GitObject::from_dir(".")?;

    git_obj.write(true)?;
    println!("{}", git_obj.hash);

    Ok(())
}
