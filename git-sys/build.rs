use make_cmd::make;
use std::env::var;
use std::fs::{copy, read_dir, DirEntry};
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

fn main() -> Result<(), Error> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = dir.join("src");
    let lib = dir.join("lib");
    let out = PathBuf::from(var("OUT_DIR").expect("OUT_DIR env var not found"));

    // update the git submodule to the latest
    Command::new("git")
        .current_dir(&dir)
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

    let headers = read_dir(&src)?
        .filter(is_toml)
        .collect::<Result<Vec<DirEntry>, _>>()?;

    Ok(())
}

fn is_toml(entry: &Result<DirEntry, Error>) -> bool {
    if let Ok(entry) = entry {
        let path = entry.path();
        let ext = path.extension();
        if ext.is_some() && ext.unwrap() == "toml" {
            return true;
        }
    }

    false
}

trait SuccessOrPanic {
    fn success_or_panic(self);
}

impl SuccessOrPanic for ExitStatus {
    fn success_or_panic(self) {
        if !self.success() {
            panic!("ran into error code {} while building", self);
        }
    }
}
