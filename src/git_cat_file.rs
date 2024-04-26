use crate::constants::GIT_DIR;
use crate::zlib_decompress::decompress;
use crate::{Error, Result};
use std::path::{Path, PathBuf};

pub fn git_cat_file(
    pretty_print: bool,
    exit_with_zero_status_if_exists: bool,
    type_obj: bool,
    size: bool,
    hash: &str,
) -> Result<()> {
    let obj_dir: PathBuf = [GIT_DIR, "objects"].iter().collect();
    let hash_loc = get_hash_object_loc(obj_dir, hash)?;

    if exit_with_zero_status_if_exists {
        if hash_loc.exists() {
            println!("Valid object");
            return Ok(());
        }
        Err(Error::InvalidGitObject)?;
    }

    let git_obj = decompress(hash_loc)?;

    // TODO: would be better with an enum
    if pretty_print {
        print!("{}", git_obj.content);
    } else if size {
        println!("{}", git_obj.size)
    } else if type_obj {
        println!("{}", git_obj.type_obj)
    }

    Ok(())

    // let object_location = get_object_location(&args[3]).unwrap();
}

pub fn get_hash_object_loc<P: AsRef<Path>>(obj_dir: P, hash: &str) -> Result<PathBuf> {
    if hash.len() != 40 {
        Err(Error::InvalidHash(hash.to_owned()))?;
    }

    let obj_path = obj_dir.as_ref();

    let (subdir, filename) = hash.split_at(2);
    let mut final_path = PathBuf::from(obj_path);
    final_path.push(subdir);
    final_path.push(filename);

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_hash_object_invalid_hash() -> Result<()> {
        let res = get_hash_object_loc("mydir", "df0");
        assert!(matches!(res, Err(Error::InvalidHash(..))));
        Ok(())
    }

    #[test]
    fn test_get_hash_object_valid_hash() -> Result<()> {
        let git_dir = ".testgit";
        let obj_dir: PathBuf = [git_dir, "objects"].iter().collect();

        fs::create_dir_all(&obj_dir)?;

        let res =
            get_hash_object_loc(&obj_dir, "0011111111111111111111111111111111111111").unwrap();
        let expected: PathBuf = [
            obj_dir.as_path(),
            "00".as_ref(),
            "11111111111111111111111111111111111111".as_ref(),
        ]
        .iter()
        .collect();
        assert_eq!(res, expected);
        fs::remove_dir_all(git_dir)?;

        Ok(())
    }
}
