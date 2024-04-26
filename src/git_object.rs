use flate2::bufread;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
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

    pub fn from_blob<P: AsRef<Path>>(file_path: P) -> Result<GitObject> {
        let file = File::open(&file_path)?;

        // TODO: probably a bad idea, files can be pretty big
        let size = file.metadata()?.len() as u32;

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

    pub fn write(&self) {
        println!("WRITING")
    }
}
