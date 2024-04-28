use sha1::{Digest, Sha1};
use std::fs::{create_dir_all, read_dir, File};
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::{Error, Result};

#[derive(Debug)]
/// content is optional because we don't need to retrieve it for every git functions
pub enum GitObjectContent {
    Blob { content: String },
    Tree { content: Vec<TreeChild> },
}

#[derive(Debug)]
pub struct TreeChild {
    pub mode: u32,
    pub git_object: GitObject,
    pub name: String,
}

#[derive(Debug)]
pub struct GitObject {
    pub size: usize,
    pub hash: String,
    pub content: GitObjectContent,
    /// Contains the bytes that are used to compute the hash
    /// Only set when loading from blob, or dir but not from hash
    pub object_bytes: Option<Vec<u8>>,
}

impl GitObject {
    pub fn get_hash_from_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        // let mut digest = Sha1::digest(&mut content); // other solution
        format!("{digest:x}")
    }

    pub fn from_hash(hash: &str) -> Result<Self> {
        if hash.len() != 40 {
            Err(Error::InvalidHash(hash.to_owned()))?;
        }
        let (subdir, filename) = hash.split_at(2);

        let location: PathBuf = [".git", "objects", subdir, filename].iter().collect();

        let file = File::open(location)?;

        let decoder = flate2::read::ZlibDecoder::new(file);
        let mut reader = BufReader::new(decoder);

        let mut header_bytes = Vec::new();
        reader.read_until(0, &mut header_bytes)?;

        header_bytes.pop();
        let header = String::from_utf8_lossy(&header_bytes).to_string();

        let (type_obj, size_str) = header
            .split_once(' ')
            .ok_or_else(|| Error::InvalidGitObject)?;

        let size = size_str
            .parse::<usize>()
            .map_err(|_| Error::InvalidGitObject)?;

        match type_obj {
            // content is actually just permission name hash for trees
            "tree" => {
                let mut buf_20 = vec![0; 20];
                let mut content_bytes = Vec::new();
                let mut content = Vec::new();
                loop {
                    reader.read_until(0, &mut content_bytes)?;
                    if content_bytes.is_empty() {
                        break;
                    }
                    content_bytes.pop(); // no need for null byte
                    let header = String::from_utf8_lossy(&content_bytes).to_string();
                    let (mode, name) = header
                        .split_once(' ')
                        .ok_or_else(|| Error::InvalidGitObject)?;

                    let mode = mode.parse::<u32>().map_err(|_| Error::InvalidGitObject)?;

                    reader.read_exact(&mut buf_20)?;

                    let mut child_hash = String::new();
                    for byte in buf_20.iter() {
                        child_hash.push_str(&format!("{:02x}", byte));
                    }
                    // let child_hash = hex::encode(&buf_20); // other possibility with hex crate

                    content.push(TreeChild {
                        mode,
                        name: name.to_string(),
                        git_object: GitObject::from_hash(&child_hash)?,
                    });
                    content_bytes.clear();
                }
                Ok(GitObject {
                    size,
                    content: GitObjectContent::Tree { content },
                    hash: hash.to_string(),
                    object_bytes: None,
                })
            }
            "blob" => {
                let mut content = String::new();
                reader.read_to_string(&mut content)?;
                Ok(GitObject {
                    size,
                    content: GitObjectContent::Blob { content },
                    hash: hash.to_string(),
                    object_bytes: None,
                })
            }
            _ => todo!(),
        }
    }

    //
    pub fn from_blob<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file = File::open(&file_path)?;

        // TODO: probably a bad idea, files can be pretty big
        let size = file.metadata()?.len() as usize;

        let mut reader = BufReader::new(file);

        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        reader.seek(SeekFrom::Start(0))?;

        let header = format!("blob {}", size);

        let mut object_bytes = header.as_bytes().to_vec();
        object_bytes.push(0);
        reader.read_to_end(&mut object_bytes)?;

        let hash = GitObject::get_hash_from_bytes(&object_bytes);

        Ok(GitObject {
            object_bytes: Some(object_bytes),
            hash,
            content: GitObjectContent::Blob { content },
            size,
        })
    }

    pub fn from_dir<P: AsRef<Path>>(dir_path: P) -> Result<Self> {
        let parent_dir = read_dir(dir_path)?;

        let mut paths = parent_dir
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect::<Vec<_>>();

        paths.sort();

        let mut content: Vec<TreeChild> = Vec::new();
        let mut content_bytes: Vec<u8> = Vec::new();

        // no handling of symbolic links
        for path in paths {
            let name = path.file_name().and_then(|e| e.to_str());

            if let Some(name) = name {
                let mode;
                let git_object;
                if path.is_file() {
                    mode = 100644;
                    git_object = GitObject::from_blob(&path)?;
                } else {
                    mode = 40000;
                    git_object = GitObject::from_dir(&path)?;
                }
                content_bytes.extend(format!("{} {}", mode, name).as_bytes());
                content_bytes.push(0);

                let git_object_hash = git_object.hash.clone();
                let hash_as_bytes = hex::decode(&git_object_hash)
                    .map_err(|e| Error::InvalidHash(git_object_hash))?;
                content_bytes.extend(hash_as_bytes);
                content.push(TreeChild {
                    mode,
                    name: name.to_string(),
                    git_object,
                });
            }
        }

        let size = content_bytes.len();
        let mut object_bytes = Vec::from(format!("tree {}", size).as_bytes());
        object_bytes.push(0);
        object_bytes.extend(&content_bytes);

        let hash = GitObject::get_hash_from_bytes(&object_bytes);

        Ok(GitObject {
            hash,
            object_bytes: Some(object_bytes),
            size,
            content: GitObjectContent::Tree { content },
        })
    }

    pub fn write(&self) -> Result<()> {
        let (subdir, filename) = self.hash.split_at(2);

        let location: PathBuf = [".agit", "objects", subdir, filename].iter().collect();

        let parent = location.parent().ok_or_else(|| Error::InvalidGitObject)?;
        create_dir_all(parent)?;
        let output = File::create(location)?;

        // write can only occur if the bytes were loaded when we got the object.
        // So it won't work if we got it from its hash, which is actually ok because
        // in this case, it already exists in the .git/objects folder
        let object_bytes = self
            .object_bytes
            .clone()
            .ok_or_else(|| Error::ObjectBytesNotLoaded)?;

        let mut encoder = flate2::write::ZlibEncoder::new(output, flate2::Compression::default());
        encoder.write_all(&object_bytes)?;

        match &self.content {
            GitObjectContent::Tree { content } => {
                for tree_child in content {
                    tree_child.git_object.write()?;
                }
                Ok(())
            }
            GitObjectContent::Blob { .. } => Ok(()),
        }
    }
    pub fn content_type(&self) -> String {
        match self.content {
            GitObjectContent::Blob { .. } => "blob".to_owned(),
            GitObjectContent::Tree { .. } => "tree".to_owned(),
        }
    }
}
