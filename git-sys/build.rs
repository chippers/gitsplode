use crate::header::Header;
use anyhow::Error;
use make_cmd::make;
use std::convert::TryFrom;
use std::env::var;
use std::fs::{copy, read_dir, write, DirEntry};
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const HEADER_PREFIX: &str = r#"
#define NO_OPENSSL
#define NO_CURL
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("src");
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

    // collect all header template files, only if none of them had an error
    let headers = read_dir(&src)?
        .filter(is_toml)
        .map(dir_entry_to_header)
        .collect::<Result<Vec<Header>, _>>()?;

    // the root module that contains the following bindings
    let mut root_module = String::new();

    for header in headers {
        // generate bindings for the header file
        let bindings = header
            .builder(&HEADER_PREFIX)
            .clang_arg(format!("-I/{}", lib.display()))
            .generate()
            .expect("unable to generate bindings");

        // write out the generated bindings to the correct file
        let out_file = out.join(format!("{}.rs", header.name));
        bindings.write_to_file(out_file)?;

        // include our generated file inside of the root module
        root_module.push_str(&format!("pub use {}::*;\n", header.name));
        root_module.push_str(&format!(
            "mod version {{ include!(concat!(env!(\"OUT_DIR\"), \"/{}.rs\")); }}\n",
            header.name
        ));
    }

    // write out the root module so that we can include it from our src/lib.rs
    write(out.join("lib.rs"), root_module)?;

    Ok(())
}

/// Check if a [`DirEntry`] is a `.toml` file
fn is_toml(entry: &Result<DirEntry, IoError>) -> bool {
    if let Ok(entry) = entry {
        let path = entry.path();
        let ext = path.extension();
        if ext.is_some() && ext.unwrap() == "toml" {
            return true;
        }
    }

    false
}

/// Convert a fetched [`DirEntry`] into a [`Header`](crate::header::Header)
fn dir_entry_to_header(entry: Result<DirEntry, IoError>) -> Result<Header, Error> {
    entry.map_err(From::from).and_then(Header::try_from)
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

/// Items to help represent a header as a TOML file
mod header {
    use anyhow::anyhow;
    use bindgen::{builder, Builder};
    use serde::Deserialize;
    use std::convert::TryFrom;
    use std::fs::{read_to_string, DirEntry};
    use std::path::Path;

    #[derive(Debug, Deserialize)]
    struct Filter {
        item: Item,
        name: String,
    }

    #[derive(Debug, Copy, Clone)]
    enum FilterType {
        Whitelist,
        Blacklist,
    }

    impl Filter {
        /// Apply a filter to a [`Builder`](bindgen::Builder)
        fn apply(&self, builder: Builder, filter_type: FilterType) -> Builder {
            // set the type of filter to be used
            let filterer = match filter_type {
                FilterType::Whitelist => match self.item {
                    Item::Function => Builder::whitelist_function,
                    Item::Type => Builder::whitelist_type,
                    Item::Item => Builder::whitelist_var,
                },
                FilterType::Blacklist => match self.item {
                    Item::Function => Builder::blacklist_function,
                    Item::Type => Builder::blacklist_type,
                    Item::Item => Builder::blacklist_item,
                },
            };

            // apply the filter to the [`Builder`](bindgen::Builder)
            filterer(builder, &self.name)
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged, rename_all = "lowercase")]
    enum Item {
        Function,
        Item,
        Type,
    }

    #[derive(Debug, Deserialize)]
    pub struct Header {
        #[serde(skip)]
        pub name: String,
        header: String,
        whitelist: Option<Vec<Filter>>,
        blacklist: Option<Vec<Filter>>,
    }

    impl TryFrom<DirEntry> for Header {
        type Error = anyhow::Error;

        fn try_from(entry: DirEntry) -> Result<Self, Self::Error> {
            let path = entry.path();
            let raw = read_to_string(&path)?;
            let mut header: Self = toml::from_str(&raw)?;

            // reject the header if it specifies both types of filter
            if header.whitelist.is_some() && header.blacklist.is_some() {
                return Err(anyhow!(
                    "found both whitelist and blacklist for {}",
                    path.display()
                ));
            }

            // set the name of the header to the filename minus the extension
            header.name = file_stem(&path)?;

            Ok(header)
        }
    }

    impl Header {
        /// Create a [`Builder`](bindgen::Builder) from a [`Header`]
        pub fn builder(&self, prefix: impl AsRef<str>) -> Builder {
            // apply the header to the builder
            let filename = format!("{}.h", &self.name);
            let content = format!("{}\n{}", prefix.as_ref(), &self.header);
            let mut builder = builder().header_contents(&filename, &content);

            // represent the filters along with their type
            let whitelist = self.whitelist.as_ref().map(|f| (f, FilterType::Whitelist));
            let blacklist = self.blacklist.as_ref().map(|f| (f, FilterType::Blacklist));

            // apply the found filters
            if let Some((filters, filter_type)) = whitelist.or(blacklist) {
                for filter in filters {
                    builder = filter.apply(builder, filter_type);
                }
            };

            builder
        }
    }

    /// Get the name of a file from a path, without the file extension
    fn file_stem(path: &Path) -> Result<String, anyhow::Error> {
        path.file_stem()
            .map(|name| name.to_os_string())
            .ok_or_else(|| anyhow!("no filename stem for header template found"))
            .and_then(|name| {
                name.into_string()
                    .map_err(|bad| anyhow!("invalid utf-8 in header template filename: {:?}", bad))
            })
    }
}
