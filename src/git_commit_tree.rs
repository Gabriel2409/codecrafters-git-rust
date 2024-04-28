use crate::git_object::{CommitObjects, GitObject};
use crate::Result;

pub fn git_commit_tree(tree_sha: &str, parent_commit_sha: &str, message: &str) -> Result<()> {
    let commit_objects = CommitObjects {
        timestamp: 1714305310,
        author_name: "Fake author".to_string(),
        author_email: "fake_author@gmail.com".to_string(),
        author_timezone: "+0200".to_string(),
        commit_msg: message.to_string(),
        tree_sha: tree_sha.to_string(),
        parents_sha: vec![parent_commit_sha.to_string()],
    };

    let git_obj = GitObject::from_commit_objects(commit_objects)?;

    git_obj.write()?;
    println!("{}", git_obj.hash);

    Ok(())
}
