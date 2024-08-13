use std::path::Path;

use git2::{Branch, BranchType, Repository};
use serde::{Deserialize, Serialize};

pub fn git_fetch(root: &Path) -> Result<(), git2::Error> {
    let repo = Repository::open(root)?;
    let mut remote = repo.find_remote("origin")?;
    let ref_specs_iter = remote.fetch_refspecs()?;
    let ref_specs: Vec<&str> = ref_specs_iter.iter().map(|spec| spec.unwrap()).collect();
    remote.fetch(&ref_specs, None, None)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct BranchInfo {
    name: String,
    commit_hash: String,
    time: i64,
    message: Vec<String>,
}

fn get_branch_info(
    branch_struct: Result<(Branch, BranchType), git2::Error>,
) -> Result<BranchInfo, git2::Error> {
    let branch = branch_struct?.0;
    let lossy_name = String::from_utf8_lossy(branch.name_bytes()?);
    let name = lossy_name
        .strip_prefix("origin/")
        .unwrap_or(&lossy_name)
        .to_owned();

    let branch_commit = branch.into_reference().peel_to_commit()?;
    let message: Vec<String> = String::from_utf8_lossy(branch_commit.message_bytes())
        .split_terminator("\n")
        .map(String::from)
        .collect();

    Ok(BranchInfo {
        name,
        commit_hash: branch_commit.id().to_string(),
        time: branch_commit.time().seconds(),
        message,
    })
}

pub fn git_list_branches(root: &Path) -> Result<Vec<BranchInfo>, git2::Error> {
    let repo = Repository::open(root)?;
    let git_branches: Result<Vec<BranchInfo>, git2::Error> = repo
        .branches(Some(BranchType::Remote))?
        // first branch seems to be HEAD, we don't want to display that to the
        // users so skip it
        .skip(1)
        .map(get_branch_info)
        .collect();
    git_branches
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use test_utils::{git_get_latest_commit, git_remote_branches, initialise_git_repo};

    use super::*;

    #[test]
    fn can_perform_git_fetch() {
        let test_git = initialise_git_repo(None);

        let remote_ref = git_get_latest_commit(&test_git.remote, "HEAD");
        let initial_ref = git_get_latest_commit(&test_git.local, "refs/remotes/origin/HEAD");
        assert_ne!(
            initial_ref.message().unwrap(),
            remote_ref.message().unwrap()
        );

        let initial_branches = git_remote_branches(&test_git.local);
        assert_eq!(initial_branches.count(), 2); // HEAD and main

        git_fetch(&test_git.dir.path().join("local")).unwrap();

        let post_fetch_ref = git_get_latest_commit(&test_git.local, "refs/remotes/origin/HEAD");
        assert_eq!(
            post_fetch_ref.message().unwrap(),
            remote_ref.message().unwrap()
        );

        let post_fetch_branches = git_remote_branches(&test_git.local);
        assert_eq!(post_fetch_branches.count(), 3); // HEAD, main and other
    }

    #[test]
    fn can_list_git_branches() {
        let test_git = initialise_git_repo(None);
        git_fetch(&test_git.dir.path().join("local")).unwrap();
        let branches = git_list_branches(&test_git.dir.path().join("local")).unwrap();
        let now_in_seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, String::from("master"));
        assert_eq!(branches[0].message, vec![String::from("Second commit")]);
        assert_eq!(branches[0].time, now_in_seconds as i64);
        assert_eq!(branches[1].name, String::from("other"));
        assert_eq!(branches[1].message, vec![String::from("Third commit")]);
        assert_eq!(branches[1].time, now_in_seconds as i64);
    }
}
