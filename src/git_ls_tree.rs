use std::path::Path;

use crate::git_object::GitObject;
use crate::{Error, Result};

pub fn git_ls_tree(name_only: bool, hash: &str) -> Result<()> {
    let git_obj = GitObject::from_hash(hash)?;

    if git_obj.type_obj != "tree" {
        Err(Error::NotATreeGitObject(hash.to_string()))?;
    }
    let git_attrs = git_obj.get_tree_attributes()?;
    if name_only {
        for git_attr in git_attrs {
            println!("{}", git_attr.name);
        }
    } else {
        for git_attr in git_attrs {
            println!(
                "{:0>6} {} {}\t{}",
                git_attr.permission, git_attr.type_obj, git_attr.hash, git_attr.name
            );
        }
    }

    Ok(())

    // let object_location = get_object_location(&args[3]).unwrap();
}
