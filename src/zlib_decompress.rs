use flate2::bufread;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;

use crate::{Error, Result};

pub struct GitObject {
    pub obj_type: String,
    pub size: u32,
    pub content: String,
}

pub fn decompress<P: AsRef<Path>>(file_path: P) -> Result<GitObject> {
    let input = BufReader::new(File::open(file_path)?);

    let mut decompressed_content = Vec::new();

    let mut decoder = bufread::ZlibDecoder::new(input);
    decoder.read_to_end(&mut decompressed_content)?;
    let full_content = String::from_utf8_lossy(&decompressed_content).to_string();

    let mut space_iter = full_content.splitn(2, |c| c == ' ');
    let obj_type = space_iter
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
        obj_type,
        size,
        content,
    })
}
