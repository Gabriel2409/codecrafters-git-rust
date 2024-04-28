use crate::git_object::{GitObject, GitObjectContent};
use crate::Result;

pub fn git_cat_file(
    pretty_print: bool,
    exit_with_zero_status_if_exists: bool,
    type_obj: bool,
    size: bool,
    hash: &str,
) -> Result<()> {
    let git_obj = GitObject::from_hash(hash)?;

    // TODO: would be better with an enum
    if exit_with_zero_status_if_exists {
        println!("Valid object");
    } else if pretty_print {
        match git_obj.content {
            GitObjectContent::Commit { content } => {
                println!("tree {}", content.tree_sha);
                for parent_sha in content.parents_sha {
                    println!("parent {}", parent_sha);
                }
                println!(
                    "author {} <{}> {} {}",
                    content.author_name,
                    content.author_email,
                    content.timestamp,
                    content.author_timezone
                );
                println!(
                    "committer {} <{}> {} {}",
                    content.author_name,
                    content.author_email,
                    content.timestamp,
                    content.author_timezone
                );
                println!();
                print!("{}", content.commit_msg);
            }

            GitObjectContent::Tree { content } => {
                // NOTE: I could implement display instead for TreeAttributes
                for tree_child in content {
                    println!(
                        "{:0>6} {} {}\t{}",
                        tree_child.mode,
                        tree_child.git_object.content_type(),
                        tree_child.git_object.hash,
                        tree_child.name
                    );
                }
            }
            GitObjectContent::Blob { content } => print!("{}", content),
        }
    } else if size {
        println!("{}", git_obj.size)
    } else if type_obj {
        println!("{}", git_obj.content_type())
    }

    Ok(())
}
