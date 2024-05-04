use std::path::Path;

use crate::{
    git_init::git_init,
    git_object::{GitObject, GitObjectContent},
    git_pack::{GitPack, UploadPackDiscovery},
};
use crate::{Error, Result};

/// see https://www.git-scm.com/docs/http-protocol
pub fn git_clone<P: AsRef<Path> + ?Sized>(repository_url: &str, directory: &P) -> Result<()> {
    // creates new repo
    git_init(directory)?;

    // retrieve upload_pack_discovery information and writes the refs and HEAD
    let upload_pack_discovery = UploadPackDiscovery::from_repository_url(repository_url)?;
    upload_pack_discovery.write_head_and_refs(directory)?;

    // retrieve the packfile content
    let pack_content =
        GitPack::create_minimal_pack_content_from_head_hash(&upload_pack_discovery.head_hash);
    let git_pack = GitPack::from_repository_url_and_pack_content(repository_url, &pack_content)?;

    // transform the packfile into the corresponding git objects
    let git_objects = git_pack.into_git_objects()?;

    // writes all objects to .git folder
    for git_object in git_objects {
        git_object.write(directory, false)?;
    }

    // retrieves the tree object corresponding to the commit where HEAD points to
    let current_commit_object =
        GitObject::from_hash(&upload_pack_discovery.head_hash, directory)?.content;
    let main_tree_sha = match current_commit_object {
        GitObjectContent::Commit { content } => content.tree_sha,
        _ => Err(Error::NotATreeGitObject)?,
    };
    let tree = GitObject::from_hash(&main_tree_sha, directory)?;

    // restores the working dir
    tree.restore_directory(directory)?;

    Ok(())
}
