use crate::git_object::GitObject;
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
        match git_obj.type_obj.as_ref() {
            "tree" => {
                // NOTE: I could implement display instead for TreeAttributes
                let git_attrs = git_obj.get_tree_attributes()?;
                for git_attr in git_attrs {
                    println!(
                        "{:0>6} {} {}\t{}",
                        git_attr.permission, git_attr.type_obj, git_attr.hash, git_attr.name
                    );
                }
            }
            _ => print!("{}", git_obj.content),
        }
    } else if size {
        println!("{}", git_obj.size)
    } else if type_obj {
        println!("{}", git_obj.type_obj)
    }

    Ok(())

    // let object_location = get_object_location(&args[3]).unwrap();
}
