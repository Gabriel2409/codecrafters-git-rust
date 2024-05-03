use crate::git_object::{GitObject, GitObjectContent};
use crate::{Error, Result};

pub fn git_ls_tree(name_only: bool, recursive: bool, long: bool, hash: &str) -> Result<()> {
    let git_obj = GitObject::from_hash(hash, ".")?;

    match git_obj.content {
        GitObjectContent::Tree { content } => {
            for tree_child in content {
                if let Some(git_obj) = tree_child.git_object {
                    if recursive && git_obj.content_type() == "tree" {
                        git_ls_tree(name_only, recursive, long, &git_obj.hash)?;
                    } else if name_only {
                        println!("{}", tree_child.name);
                    } else if long {
                        println!(
                            "{:0>6} {} {} {:>8}\t{}",
                            tree_child.mode,
                            git_obj.content_type(),
                            git_obj.hash,
                            git_obj.size,
                            tree_child.name
                        );
                    } else {
                        println!(
                            "{:0>6} {} {}\t{}",
                            tree_child.mode,
                            git_obj.content_type(),
                            git_obj.hash,
                            tree_child.name
                        );
                    }
                } else {
                    Err(Error::TreeChildNotLoaded)?;
                }
            }
        } // Return Ok(()) on success
        _ => Err(Error::NotATreeGitObject(hash.to_string()))?,
    }

    Ok(())
}
