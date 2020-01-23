use bstr::BStr;
use git_sys::git_version_string;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::sync::Once;

#[derive(Debug)]
pub struct Repository(git_sys::repository);

impl Repository {
    pub fn init(gitdir: CString, worktree: CString) -> Self {
        let mut repo = MaybeUninit::<git_sys::repository>::uninit();
        unsafe {
            git_sys::repo_init(
                repo.as_mut_ptr(),
                dbg!(gitdir.as_ptr()),
                dbg!(worktree.as_ptr()),
            );

            Self(repo.assume_init())
        }
    }

    pub fn gitdir(&self) -> &BStr {
        let cstr = unsafe { CStr::from_ptr(self.0.gitdir) };
        cstr.to_bytes().into()
    }
}

static INIT_VERSION: Once = Once::new();
static mut VERSION: &str = "";
pub fn version() -> &'static str {
    unsafe {
        INIT_VERSION.call_once(|| {
            VERSION = CStr::from_ptr(git_version_string.as_ptr())
                .to_str()
                .expect("invalid utf-8 found in git version string")
        });

        VERSION
    }
}
