use git_sys::git_version_string;
use std::ffi::CStr;
use std::sync::Once;

pub struct Git {
    repository: *mut git_sys::repository,
}

static INIT_GIT: Once = Once::new();
static mut GIT: Git = Git {
    repository: std::ptr::null_mut(),
};

/// FIXME: is this unsafe? essentially potentially exposing a static !Send ptr across threads
pub fn init() -> &'static Git {
    unsafe {
        INIT_GIT.call_once(|| {
            git_sys::git_setup_gettext();
            git_sys::attr_start();
            git_sys::initialize_the_repository();

            GIT.repository = git_sys::the_repository;
        });

        &GIT
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
