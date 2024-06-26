use sha1::{Digest, Sha1};
use std::fs::{create_dir_all, read_dir, File};
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::{Error, Result};

#[derive(Debug)]
/// Content of a given GitObject
pub enum GitObjectContent {
    Blob {
        content: String,
    },
    Tree {
        content: Vec<TreeChild>,
    },
    Commit {
        content: CommitObjects,
    },
    /// only annotated tags are stored as objects
    Tag {
        /// For now we stores the full content as a string
        /// but we could do like the commit object and store it as
        /// its constituents
        content: String,
    },
}

/// Tree points to a git object but also have additional information
/// such as the name and the mode of said object
#[derive(Debug)]
pub struct TreeChild {
    /// Different values for dir and blob (100644 for files, 40000 for dirs)
    pub mode: u32,
    /// Do not store the full child object when unnecessary
    pub git_object: Option<GitObject>,
    /// hash of underlying git object
    pub hash: String,
    /// name of the file
    pub name: String,
}

impl TreeChild {
    /// Creates a TreeChild without loading the underlying git_object
    pub fn from_reader<R: BufRead>(reader: &mut R) -> Result<Option<Self>> {
        let mut buf_20 = vec![0; 20];
        let mut content_bytes = Vec::new();
        reader.read_until(0, &mut content_bytes)?;
        if content_bytes.is_empty() {
            return Ok(None);
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

        Ok(Some(TreeChild {
            mode,
            name: name.to_string(),
            hash: child_hash.to_string(),
            git_object: None,
        }))
    }

    /// uses name and child content to recreate it in the target dir
    pub fn restore_directory<P: AsRef<Path> + ?Sized>(self, parent_dir: &P) -> Result<()> {
        std::fs::create_dir_all(parent_dir.as_ref())?;
        let git_object = self.git_object.ok_or_else(|| Error::TreeChildNotLoaded)?;

        let child = parent_dir.as_ref().join(self.name);

        match git_object.content {
            GitObjectContent::Blob { content } => {
                std::fs::write(child, content)?;
            }
            GitObjectContent::Tree { content } => {
                for tree_child in content {
                    tree_child.restore_directory(&child)?;
                }
            }
            _ => Err(Error::InvalidGitObject)?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CommitObjects {
    pub timestamp: u32,
    // we'll have author and committer be the same
    pub author_email: String,
    pub author_name: String,
    pub author_timezone: String,
    // support for multiple parents. But in our case, we will have only 1
    pub parents_sha: Vec<String>,
    pub tree_sha: String,
    pub commit_msg: String,
}

impl CommitObjects {
    /// Parses a string (decoded by zlib) to create the objects
    pub fn from_content(content: &str) -> Result<Self> {
        // TODO: may fail if empty commit msg
        let (beginning, commit_msg) = content
            .split_once("\n\n")
            .ok_or_else(|| Error::InvalidGitObject)?;
        let commit_msg = commit_msg.to_string();
        let mut timestamp = 0;
        let mut author_email = String::from("");
        let mut author_name = String::from("");
        let mut author_timezone = String::from("");
        let mut parents_sha = Vec::new();
        let mut tree_sha = String::from("");

        for line in beginning.lines() {
            let (head, tail) = line
                .split_once(' ')
                .ok_or_else(|| Error::InvalidGitObject)?;

            match head {
                "tree" => {
                    tree_sha = tail.to_string();
                }
                "parent" => {
                    parents_sha.push(tail.to_string());
                }
                "author" => {
                    let mut author_info = tail.split(' ').collect::<Vec<_>>();

                    author_timezone = author_info
                        .pop()
                        .ok_or_else(|| Error::InvalidGitObject)?
                        .to_string();
                    timestamp = author_info
                        .pop()
                        .ok_or_else(|| Error::InvalidGitObject)?
                        .parse::<u32>()
                        .map_err(|_| Error::InvalidGitObject)?;
                    let author_email_enclosing =
                        author_info.pop().ok_or_else(|| Error::InvalidGitObject)?;

                    author_email =
                        author_email_enclosing[1..author_email_enclosing.len() - 1].to_string();
                    author_name = author_info.join(" ");
                }

                "committer" => {
                    // same as author for us
                }
                _ => {}
            }
        }
        Ok(CommitObjects {
            timestamp,
            author_name,
            author_email,
            author_timezone,
            parents_sha,
            tree_sha,
            commit_msg,
        })
    }
}

#[derive(Debug)]
pub struct GitObject {
    pub size: usize,
    pub hash: String,
    pub content: GitObjectContent,
    /// Contains the bytes that are used to compute the hash
    /// Only set when loading from blob, or dir but not from hash
    /// to avoid unnecessary storage
    pub object_bytes: Option<Vec<u8>>,
}

impl GitObject {
    /// Helper function to compute the hash of a vector of bytes
    pub fn get_hash_from_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        // let mut digest = Sha1::digest(&mut content); // other solution
        format!("{digest:x}")
    }

    pub fn find_hash_branch_or_tag_points_to<P: AsRef<Path> + ?Sized>(
        name: &str,
        repository_directory: &P,
    ) -> Result<String> {
        let branch_location: PathBuf = [".git", "refs", "heads", name].iter().collect();
        let branch_location = repository_directory.as_ref().join(branch_location);

        let tag_location: PathBuf = [".git", "refs", "tags", name].iter().collect();
        let tag_location = repository_directory.as_ref().join(tag_location);

        let location = if branch_location.exists() {
            branch_location
        } else if tag_location.exists() {
            tag_location
        } else {
            return Err(Error::InvalidBranchOrTag);
        };

        let mut content = std::fs::read_to_string(location)?;
        // removes the \n
        content.pop();

        Ok(content)
    }

    /// Create a git object based on the hash in hexadecimal format.
    /// Under the hood, locates the file in the .git/objects folder of the
    /// repository_directory
    /// Note: If given hash is not 40 chars, we instead try to get it from branch ref
    /// first and tag second
    pub fn from_hash<P: AsRef<Path> + ?Sized>(
        hash: &str,
        repository_directory: &P,
    ) -> Result<Self> {
        if hash.len() != 40 {
            let hash = Self::find_hash_branch_or_tag_points_to(hash, repository_directory)?;
            if hash.len() != 40 {
                Err(Error::InvalidHash(hash.clone()))?;
            }

            return Self::from_hash(&hash, repository_directory);
        }
        let (subdir, filename) = hash.split_at(2);

        let location: PathBuf = [".git", "objects", subdir, filename].iter().collect();
        let location = repository_directory.as_ref().join(location);

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
                let mut content = Vec::new();
                loop {
                    let tree_child = TreeChild::from_reader(&mut reader)?;
                    match tree_child {
                        None => break,
                        Some(mut tree_child) => {
                            // loads the underlying git object
                            tree_child.git_object = Some(GitObject::from_hash(
                                &tree_child.hash,
                                repository_directory,
                            )?);
                            content.push(tree_child);
                        }
                    }
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
            "commit" => {
                let mut content = String::new();
                reader.read_to_string(&mut content)?;

                let commit_objects = CommitObjects::from_content(&content)?;

                Ok(GitObject {
                    size,
                    hash: hash.to_string(),
                    object_bytes: None,
                    content: GitObjectContent::Commit {
                        content: commit_objects,
                    },
                })
            }

            "tag" => {
                let mut content = String::new();
                reader.read_to_string(&mut content)?;
                Ok(GitObject {
                    size,
                    content: GitObjectContent::Tag { content },
                    hash: hash.to_string(),
                    object_bytes: None,
                })
            }

            // are there actually other objects?
            _ => todo!(),
        }
    }

    /// Creates a blob object from content bytes (header not included)
    pub fn from_blob_content_bytes(content_bytes: Vec<u8>) -> Result<Self> {
        let size = content_bytes.len();

        let header = format!("blob {}", size);
        let mut object_bytes = header.as_bytes().to_vec();
        object_bytes.push(0);
        object_bytes.extend(&content_bytes);

        let hash = GitObject::get_hash_from_bytes(&object_bytes);
        let content = String::from_utf8(content_bytes).map_err(|_| Error::InvalidSmartHttpRes)?;

        Ok(GitObject {
            object_bytes: Some(object_bytes),
            hash,
            content: GitObjectContent::Blob { content },
            size,
        })
    }

    /// Creates a tree object from content bytes (header not included)
    pub fn from_tree_content_bytes(content_bytes: Vec<u8>) -> Result<Self> {
        let size = content_bytes.len();

        let header = format!("tree {}", size);
        let mut object_bytes = header.as_bytes().to_vec();
        object_bytes.push(0);
        object_bytes.extend(&content_bytes);
        let hash = GitObject::get_hash_from_bytes(&object_bytes);

        let mut reader = BufReader::new(&content_bytes[..]);
        let mut content = Vec::new();
        loop {
            let tree_child = TreeChild::from_reader(&mut reader)?;
            match tree_child {
                None => break,
                Some(tree_child) => {
                    content.push(tree_child);
                }
            }
        }
        Ok(GitObject {
            size,
            content: GitObjectContent::Tree { content },
            hash: hash.to_string(),
            object_bytes: Some(object_bytes),
        })
    }

    /// Creates a commit object from content bytes (header not included)
    pub fn from_commit_content_bytes(content_bytes: Vec<u8>) -> Result<Self> {
        let size = content_bytes.len();

        let header = format!("commit {}", size);
        let mut object_bytes = header.as_bytes().to_vec();
        object_bytes.push(0);
        object_bytes.extend(&content_bytes);

        let hash = GitObject::get_hash_from_bytes(&object_bytes);
        let content = String::from_utf8(content_bytes).map_err(|_| Error::InvalidSmartHttpRes)?;

        let commit_objects = CommitObjects::from_content(&content)?;

        Ok(GitObject {
            object_bytes: Some(object_bytes),
            hash,
            content: GitObjectContent::Commit {
                content: commit_objects,
            },
            size,
        })
    }

    /// Creates a tag object from content bytes (header not included)
    pub fn from_tag_content_bytes(content_bytes: Vec<u8>) -> Result<Self> {
        let size = content_bytes.len();

        let header = format!("tag {}", size);
        let mut object_bytes = header.as_bytes().to_vec();
        object_bytes.push(0);
        object_bytes.extend(&content_bytes);

        let hash = GitObject::get_hash_from_bytes(&object_bytes);
        let content = String::from_utf8(content_bytes).map_err(|_| Error::InvalidSmartHttpRes)?;

        Ok(GitObject {
            object_bytes: Some(object_bytes),
            hash,
            content: GitObjectContent::Tag { content },
            size,
        })
    }

    /// Creates a git blob object from a file.
    pub fn from_blob<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file = File::open(&file_path)?;
        // let size = file.metadata()?.len() as usize;

        let mut reader = BufReader::new(file);

        // let mut content = String::new();
        // reader.read_to_string(&mut content)?;
        // reader.seek(SeekFrom::Start(0))?;

        let mut content_bytes = Vec::new();
        reader.read_to_end(&mut content_bytes)?;
        Self::from_blob_content_bytes(content_bytes)
    }

    /// Creates a tree object from a directory
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
                // Ignore git folder
                if name == ".git" {
                    continue;
                }
                let mode;
                let git_object;
                // TODO: support for symbolic link
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
                    .map_err(|_| Error::InvalidHash(git_object_hash.clone()))?;
                content_bytes.extend(hash_as_bytes);
                content.push(TreeChild {
                    mode,
                    name: name.to_string(),
                    hash: git_object_hash,
                    git_object: Some(git_object),
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

    pub fn from_commit_objects(commit_objects: CommitObjects) -> Result<Self> {
        let mut content_bytes: Vec<u8> = Vec::new();

        // Should i use the hash as a str or bytes here?
        content_bytes.extend(Vec::from(
            format!("tree {}\n", commit_objects.tree_sha).as_bytes(),
        ));

        for parent_sha in &commit_objects.parents_sha {
            content_bytes.extend(Vec::from(format!("parent {}\n", parent_sha).as_bytes()));
        }

        content_bytes.extend(Vec::from(
            format!(
                "author {} <{}> {} {}\n",
                commit_objects.author_name,
                commit_objects.author_email,
                commit_objects.timestamp,
                commit_objects.author_timezone
            )
            .as_bytes(),
        ));

        content_bytes.extend(Vec::from(
            format!(
                "committer {} <{}> {} {}\n\n",
                commit_objects.author_name,
                commit_objects.author_email,
                commit_objects.timestamp,
                commit_objects.author_timezone
            )
            .as_bytes(),
        ));

        content_bytes.extend(Vec::from(
            format!("{}\n", commit_objects.commit_msg).as_bytes(),
        ));

        let size = content_bytes.len();

        let mut object_bytes = Vec::from(format!("commit {}", size));
        object_bytes.push(0);
        object_bytes.extend(&content_bytes);

        let hash = GitObject::get_hash_from_bytes(&object_bytes);

        Ok(GitObject {
            size,
            hash,
            content: GitObjectContent::Commit {
                content: commit_objects,
            },
            object_bytes: Some(object_bytes),
        })
    }

    /// Restores a tree object in a directory
    pub fn restore_directory<P: AsRef<Path> + ?Sized>(self, directory: &P) -> Result<()> {
        match self.content {
            GitObjectContent::Tree { content } => {
                for tree_child in content {
                    tree_child.restore_directory(directory)?;
                }
            }
            _ => Err(Error::NotATreeGitObject)?,
        }
        Ok(())
    }

    /// Writes the object to the .git folder of the repository_directory
    /// If recursive is set to true, tree objects will also write all their child objects
    /// Note that recursive has no effects on commit objects as they only store the
    /// sha of the main tree and not the actual object itself in this implementation
    pub fn write<P: AsRef<Path> + ?Sized>(
        &self,
        repository_directory: &P,
        recursive: bool,
    ) -> Result<()> {
        let (subdir, filename) = self.hash.split_at(2);

        let location: PathBuf = [".git", "objects", subdir, filename].iter().collect();
        let location = repository_directory.as_ref().join(location);

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

        // only applies to trees
        if recursive {
            if let GitObjectContent::Tree { content } = &self.content {
                for tree_child in content {
                    tree_child
                        .git_object
                        .as_ref()
                        .ok_or_else(|| Error::TreeChildNotLoaded)?
                        .write(repository_directory, true)?;
                }
            }
        }
        Ok(())
    }

    /// Blob => "blob", Tree => "tree", Commit => "commit"
    pub fn content_type(&self) -> String {
        match self.content {
            GitObjectContent::Blob { .. } => "blob".to_owned(),
            GitObjectContent::Tree { .. } => "tree".to_owned(),
            GitObjectContent::Commit { .. } => "commit".to_owned(),
            GitObjectContent::Tag { .. } => "tag".to_owned(),
        }
    }
}
