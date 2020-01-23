use git::Repository;
use std::ffi::CString;

fn main() {
    let version = git::version();
    println!("git version: {}", version);

    let gitdir = CString::new("./.git").unwrap();
    let worktree = CString::new(".").unwrap();
    let repo = Repository::init(gitdir, worktree);

    dbg!(&repo);

    println!("gitdir: {}", repo.gitdir().display())
}
