use sha1::{Digest, Sha1};
use std::fs::create_dir_all;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::{Error, Result};

// #[derive(Debug)]
// pub struct GitObjectAttributes {
//     hash: String,
//     type_obj: String, // could make an enum instead
//     permission: String,
//     name: String,
// }

#[derive(Debug)]
pub struct GitObject {
    pub type_obj: String,
    pub size: usize,
    pub content: String,
    pub hash: String,
}

impl GitObject {
    pub fn from_hash(hash: &str) -> Result<Self> {
        if hash.len() != 40 {
            Err(Error::InvalidHash(hash.to_owned()))?;
        }
        let (subdir, filename) = hash.split_at(2);

        let location: PathBuf = [".git", "objects", subdir, filename].iter().collect();

        let file = File::open(location)?;

        // TODO: could be improved, maybe with read_until \0
        let decoder = flate2::read::ZlibDecoder::new(file);
        let mut buf = BufReader::new(decoder);

        let mut header_bytes = Vec::new();
        buf.read_until(0, &mut header_bytes)?;

        header_bytes.pop();
        let header = String::from_utf8_lossy(&header_bytes).to_string();

        let (type_obj, size_str) = header
            .split_once(' ')
            .ok_or_else(|| Error::InvalidGitObject)?;

        let size = size_str
            .parse::<usize>()
            .map_err(|_| Error::InvalidGitObject)?;

        // TODO: It seems the trees are actually not
        // hashed in the same way
        if type_obj == "tree" {
            todo!("NOT YET IMPLEMENTED");
        }

        let mut content = String::new();
        buf.read_to_string(&mut content)?;

        Ok(GitObject {
            type_obj: type_obj.to_string(),
            size,
            content,
            hash: hash.to_string(),
        })
    }

    pub fn from_blob<P: AsRef<Path>>(file_path: P) -> Result<GitObject> {
        let file = File::open(&file_path)?;

        // TODO: probably a bad idea, files can be pretty big
        let size = file.metadata()?.len() as usize;

        let mut reader = BufReader::new(file);

        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        reader.seek(SeekFrom::Start(0))?;

        let header = format!("blob {}", size);

        let mut bytes = header.as_bytes().to_vec();
        bytes.push(0);

        reader.read_to_end(&mut bytes)?;
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        // let mut digest = Sha1::digest(&mut content); // other solution
        let hash = format!("{digest:x}");

        Ok(GitObject {
            type_obj: "blob".to_string(),
            hash,
            content,
            size,
        })
    }

    pub fn write(&self) -> Result<()> {
        let (subdir, filename) = self.hash.split_at(2);

        let location: PathBuf = [".git", "objects", subdir, filename].iter().collect();

        let parent = location.parent().ok_or_else(|| Error::InvalidGitObject)?;
        create_dir_all(parent)?;
        let output = File::create(location)?;

        let header = format!("blob {}", self.size);

        let mut bytes = header.as_bytes().to_vec();
        bytes.push(0);
        bytes.extend(self.content.as_bytes());

        let mut encoder = flate2::write::ZlibEncoder::new(output, flate2::Compression::default());
        encoder.write_all(&bytes)?;

        Ok(())
    }

    // pub fn get_tree_links(&self) -> Result<Vec<GitObjectAttributes>> {
    //     if self.type_obj != "tree".to_string() {
    //         Err(Error::NotATreeGitObject(self.hash.to_string()))?;
    //     }
    //
    //     let linked = self
    //         .content
    //         .lines()
    //         .try_fold(Vec::new(), |mut attributes, line| {
    //             let parts = line.trim().split_whitespace().collect::<Vec<&str>>();
    //             match parts.len() {
    //                 4 => {
    //                     attributes.push(GitObjectAttributes {
    //                         permission: parts[0].to_string(),
    //                         type_obj: parts[1].to_string(),
    //                         hash: parts[2].to_string(),
    //                         name: parts[3].to_string(),
    //                     });
    //                     Ok(attributes)
    //                 }
    //                 _ => Err(Error::InvalidGitObject),
    //             }
    //         })?;
    //     Ok(linked)
    // }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn aa() {
//         let git_obj = GitObject::from_hash("21423a4e94e96cc4027a9aed1a6b2ce0bd4c5972").unwrap();
//         dbg!(&git_obj);
//         let b = git_obj.get_tree_links().unwrap();
//         dbg!(b);
//         panic!("AA");
//     }
// }
