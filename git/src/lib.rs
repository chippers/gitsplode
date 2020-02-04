use errno::{errno, Errno};
use git_sys::git_version_string;
use std::ffi::{CStr, CString, NulError};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::raw::c_int;
use std::path::Path;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use thiserror::Error;

#[derive(Debug)]
pub struct Git {
    repository: *mut git_sys::repository,
    startup_info: *mut git_sys::startup_info,
}

pub fn init() -> Option<Git> {
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        None
    } else {
        let (repository, startup_info) = unsafe {
            git_sys::initialize_the_repository();
            (git_sys::the_repository, git_sys::startup_info)
        };

        Some(Git {
            repository,
            startup_info,
        })
    }
}

#[derive(Debug, Error)]
pub enum RepoInitError {
    #[error("path encoding was not valid utf-8")]
    Encoding,
    #[error("nul byte found: {0}")]
    Nul(NulError),
    #[error("invalid return status code, errno is set to: {0}")]
    Errno(Errno),
}

#[derive(Debug)]
pub struct Repository(git_sys::repository);

impl Repository {
    pub fn init(
        gitdir: impl AsRef<Path>,
        worktree: impl AsRef<Path>,
    ) -> Result<Self, RepoInitError> {
        let gitdir = Self::path_to_cstring(gitdir)?;
        let worktree = Self::path_to_cstring(worktree)?;
        let mut repo = MaybeUninit::<git_sys::repository>::uninit();

        unsafe {
            git_sys::repo_init(repo.as_mut_ptr(), gitdir.as_ptr(), worktree.as_ptr())
                .as_result(|| repo.assume_init())
        }
        .map(Repository)
        .map_err(RepoInitError::Errno)
    }

    fn path_to_cstring(path: impl AsRef<Path>) -> Result<CString, RepoInitError> {
        CString::new(path.as_ref().to_str().ok_or(RepoInitError::Encoding)?)
            .map_err(RepoInitError::Nul)
    }
}

pub struct Revisions<'repo> {
    pub inner: git_sys::rev_info,
    _marker: PhantomData<&'repo Repository>,
}

impl<'repo> Revisions<'repo> {
    pub fn init(repo: &mut Repository) -> Revisions<'_> {
        let mut rev_info = MaybeUninit::<git_sys::rev_info>::uninit();
        let rev_info = unsafe {
            git_sys::repo_init_revisions(&mut repo.0, rev_info.as_mut_ptr(), null());
            rev_info.assume_init()
        };

        Revisions {
            inner: rev_info,
            _marker: Default::default(),
        }
    }

    pub fn add_head_to_pending(&mut self) {
        //let r: *mut git_sys::rev_info = &mut self.inner;
        //unsafe { git_sys::add_head_to_pending(r) }
    }
}

trait StatusToResult {
    fn as_result<T, F>(self, ok: F) -> Result<T, Errno>
    where
        F: Fn() -> T;
}

impl StatusToResult for c_int {
    fn as_result<T, F>(self, ok: F) -> Result<T, Errno>
    where
        F: Fn() -> T,
    {
        match self {
            0 => Ok(ok()),
            _ => Err(errno()),
        }
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
