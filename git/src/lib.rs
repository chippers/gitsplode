use git_sys::git_version_string;
use std::borrow::Cow;
use std::ffi::{CStr, CString, OsStr};
use std::fmt;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Once;

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

    pub fn gitdir(&self) -> Cow<'_, Path> {
        self.c_char_to_path(self.0.gitdir)
    }

    pub fn commondir(&self) -> Cow<'_, Path> {
        self.c_char_to_path(self.0.commondir)
    }

    fn c_char_to_path(&self, ptr: *const c_char) -> Cow<'_, Path> {
        let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
        let path;

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            path = Cow::Borrowed(OsStr::from_bytes(bytes).as_ref());
        }

        #[cfg(windows)]
        {
            use std::path::PathBuf;
            match bytes.to_os_str_lossy() {
                Cow::Owned(string) => path = Cow::Owned(PathBuf::from(string)),
                Cow::Borrowed(s) => path = Cow::Borrowed(s.as_ref()),
            }
        }

        path
    }
}

impl fmt::Debug for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Repository")
            .field("gitdir", &self.gitdir())
            .field("commondir", &self.commondir())
            .finish()
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
