use std::{
    io::{BufReader, Read},
    path::Path,
};

use crate::{
    git_init::git_init,
    git_object::{GitObject, GitObjectContent},
    git_pack::{GitPack, UploadPackDiscovery},
};
use crate::{Error, Result};

/// see https://www.git-scm.com/docs/http-protocol
pub fn git_clone(repository_url: &str, directory: &str) -> Result<()> {
    git_init(directory)?;
    let upload_pack_discovery = UploadPackDiscovery::from_repository_url(repository_url)?;
    dbg!(&upload_pack_discovery);
    upload_pack_discovery.write_head_and_refs(directory)?;

    let pack_content =
        GitPack::create_minimal_pack_content_from_head_hash(&upload_pack_discovery.head_hash);
    let git_pack = GitPack::from_repository_url_and_pack_content(repository_url, &pack_content)?;
    let git_objects = git_pack.into_git_objects(directory)?;

    let current_commit_object =
        GitObject::from_hash(&upload_pack_discovery.head_hash, directory)?.content;
    let main_tree_sha = match current_commit_object {
        GitObjectContent::Commit { content } => content.tree_sha,
        _ => panic!("AA"),
    };
    let tree = GitObject::from_hash(&main_tree_sha, directory)?;
    match tree.content {
        GitObjectContent::Tree { content } => {
            for tree_child in content {
                tree_child.restore_content(directory)?;
            }
        }
        _ => panic!("BB"),
    }

    Ok(())
}
