use std::{fs, path::Path};

use crate::Result;

pub fn git_init<P: ?Sized + AsRef<Path>>(directory_path: &P) -> Result<()> {
    fs::create_dir_all(directory_path)?;
    let git_dir = directory_path.as_ref().join(".git");
    fs::create_dir(&git_dir)?;

    let git_object_dir = git_dir.join("objects");
    fs::create_dir(git_object_dir)?;

    let git_refs_dir = git_dir.join("refs");
    fs::create_dir(git_refs_dir)?;

    let head_file = git_dir.join("HEAD");

    fs::write(head_file, "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}
