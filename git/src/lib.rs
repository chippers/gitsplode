use git_sys::git_version_string;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InitError {
    #[error("this process has already initialized git")]
    Initialized,
    #[error("the current working directory is not a git repository")]
    NonGit,
}

#[derive(Debug)]
pub struct Git {
    repository: *mut git_sys::repository,
    startup_info: *mut git_sys::startup_info,
}

/// The Current Working Directory **Must** be inside a repository.
pub fn init() -> Result<Git, InitError> {
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        Err(InitError::Initialized)
    } else {
        let mut nongit_ok = 0;
        let (repository, startup_info) = unsafe {
            // initializes the global the_repository variable
            git_sys::initialize_the_repository();

            // sets up some globals with information about the git directory
            git_sys::setup_git_directory_gently(&mut nongit_ok);

            git_sys::validate_cache_entries((*git_sys::the_repository).index);

            // pass back unsafe globals to the safe block
            (git_sys::the_repository, git_sys::startup_info)
        };

        // we dont have a use for being in a non-git directory
        if nongit_ok != 0 {
            return Err(InitError::NonGit);
        }

        Ok(Git {
            repository,
            startup_info,
        })
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
