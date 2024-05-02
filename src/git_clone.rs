use std::{
    io::{BufReader, Read},
    path::Path,
};

use crate::{
    git_object::{GitObject, GitObjectContent},
    git_pack::{GitPack, UploadPackDiscovery},
};
use crate::{Error, Result};

/// see https://www.git-scm.com/docs/http-protocol
pub fn git_clone<P: AsRef<Path>>(repository_url: &str, directory: P) -> Result<()> {
    let upload_pack_discovery = UploadPackDiscovery::from_repository_url(repository_url)?;
    dbg!(&upload_pack_discovery);

    let pack_content =
        GitPack::create_minimal_pack_content_from_head_hash(&upload_pack_discovery.head_hash);
    let git_pack = GitPack::from_repository_url_and_pack_content(repository_url, &pack_content)?;
    let git_objects = git_pack.into_git_objects()?;

    Ok(())
}
