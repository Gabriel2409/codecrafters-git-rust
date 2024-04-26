use flate2::bufread;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;

use crate::{Error, Result};

pub struct GitObject {
    pub type_obj: String,
    pub size: u32,
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

        // pub fn decompress<P: AsRef<Path>>(file_path: P) -> Result<GitObject> {
        let input = BufReader::new(File::open(location)?);

        let mut decompressed_content = Vec::new();

        // TODO: could be improved, maybe with read_until \0
        let mut decoder = bufread::ZlibDecoder::new(input);
        decoder.read_to_end(&mut decompressed_content)?;
        let full_content = String::from_utf8_lossy(&decompressed_content).to_string();

        let mut space_iter = full_content.splitn(2, |c| c == ' ');
        let type_obj = space_iter
            .next()
            .ok_or_else(|| Error::InvalidGitObject)?
            .to_string();

        let mut null_iter = space_iter
            .next()
            .ok_or_else(|| Error::InvalidGitObject)?
            .splitn(2, |c| c == '\0');

        let size: u32 = null_iter
            .next()
            .ok_or_else(|| Error::InvalidGitObject)?
            .parse()
            .map_err(|_| Error::InvalidGitObject)?;

        let content = null_iter
            .next()
            .ok_or_else(|| Error::InvalidGitObject)?
            .to_string();

        Ok(GitObject {
            type_obj,
            size,
            content,
            hash: hash.to_string(),
        })
    }
}
