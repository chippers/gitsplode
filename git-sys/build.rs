use self::Filter::*;
use bindgen::Builder;
use make_cmd::make;
use std::env::var;
use std::fs::copy;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

enum Filter {
    Function,
    // Type,
    Var,
}

const WHITELIST: &[(Filter, &str)] = &[
    //
    // cache
    (Var, "startup_info"),
    (Function, "validate_cache_entries"),
    (Function, "setup_git_directory_gently"),
    //
    // repository
    (Var, "the_repository"),
    (Function, "initialize_the_repository"),
    (Function, "repo_init"),
    //
    // revisions
    (Function, "add_head_to_pending"),
    (Function, "repo_init_revisions"),
    //
    // version
    (Var, "git_version_string"),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let header = root.join("src").join("git-sys.h");
    let lib = root.join("lib");
    let out = PathBuf::from(var("OUT_DIR").expect("OUT_DIR env var not found"));

    // update the git submodule to the latest
    Command::new("git")
        .current_dir(&root)
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .status()?
        .success_or_panic();

    // create the configure tool
    make()
        .current_dir(&lib)
        .arg("configure")
        .status()?
        .success_or_panic();

    // cache the autoconf configuration generated
    let cache = out.join("configure.cache");

    // run the configuration with our parameters
    Command::new("./configure")
        .current_dir(&lib)
        .arg(format!("--cache-file={}", cache.display()))
        .arg("NO_OPENSSL=1")
        .arg("NO_CURL=1")
        .status()?
        .success_or_panic();

    // run the actual build
    make()
        .current_dir(&lib)
        .arg(format!("-j{}", num_cpus::get()))
        .status()?
        .success_or_panic();

    // copy over the generated static libraries to our output directory
    copy(lib.join("libgit.a"), out.join("libgit.a"))?;
    copy(lib.join("vcs-svn").join("lib.a"), out.join("libvcssvn.a"))?;
    copy(lib.join("xdiff").join("lib.a"), out.join("libxdiff.a"))?;

    // link the libraries that we just built
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=git");
    println!("cargo:rustc-link-lib=static=vcssvn");
    println!("cargo:rustc-link-lib=static=xdiff");

    // and link the system dependencies
    println!("cargo:rustc-link-lib=z");

    // apply our whitelist filters to a [`Builder`](bindgen::Builder)
    let mut builder = Builder::default();
    for (filter, name) in WHITELIST {
        builder = match filter {
            Function => builder.whitelist_function(name),
            // Type => builder.whitelist_type(name),
            Var => builder.whitelist_var(name),
        }
    }

    // generate bindings for our header
    let bindings = builder
        .header(header.display().to_string())
        .clang_arg(format!("-I/{}", lib.display()))
        .rustfmt_bindings(true)
        .generate()
        .expect("unable to generate bindings");

    // write out the generated bindings
    bindings.write_to_file(out.join("lib.rs"))?;

    Ok(())
}

/// Helper trait to panic when a command doesn't return a success code
trait SuccessOrPanic {
    fn success_or_panic(self);
}

impl SuccessOrPanic for ExitStatus {
    /// Panic if our [`ExitStatus`] isn't successful
    fn success_or_panic(self) {
        if !self.success() {
            panic!("ran into error code {} while building", self);
        }
    }
}
