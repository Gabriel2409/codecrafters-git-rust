use crate::git_object::{GitObject, GitObjectContent};
use crate::{Error, Result};

pub fn git_ls_tree(name_only: bool, recursive: bool, long: bool, hash: &str) -> Result<()> {
    let git_obj = GitObject::from_hash(hash)?;

    match git_obj.content {
        GitObjectContent::Tree { content } => {
            for tree_child in content {
                if recursive && tree_child.git_object.content_type() == "tree" {
                    git_ls_tree(name_only, recursive, long, &tree_child.git_object.hash)?;
                } else if name_only {
                    println!("{}", tree_child.name);
                } else if long {
                    println!(
                        "{:0>6} {} {} {:>8}\t{}",
                        tree_child.mode,
                        tree_child.git_object.content_type(),
                        tree_child.git_object.hash,
                        tree_child.git_object.size,
                        tree_child.name
                    );
                } else {
                    println!(
                        "{:0>6} {} {}\t{}",
                        tree_child.mode,
                        tree_child.git_object.content_type(),
                        tree_child.git_object.hash,
                        tree_child.name
                    );
                }
            }
        } // Return Ok(()) on success
        _ => Err(Error::NotATreeGitObject(hash.to_string()))?,
    }

    Ok(())
}
