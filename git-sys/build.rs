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
#include <git-compat-util.h>
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bindings = root.join("src").join("bindings");
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
    let headers = read_dir(&bindings)?
        .filter(is_toml)
        .map(dir_entry_to_header)
        .collect::<Result<Vec<Header>, _>>()?;

    let mut imports = String::new();
    let mut mods = String::new();

    // open bindings module
    mods.push_str("pub mod bindings {\n");

    for header in headers {
        // generate bindings for the header file
        let bindings = header
            .builder()
            .clang_arg(format!("-I/{}", lib.display()))
            .generate()
            .expect("unable to generate bindings");

        // write out the generated bindings to the correct file
        let out_file = out.join(format!("{}.rs", header.name));
        bindings.write_to_file(out_file)?;

        // create a nested module to house the generated binding code
        mods.push_str(&format!(
            "pub mod {0} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{0}.rs\")); }}\n",
            header.name
        ));

        // import all our items from our generated binding module to the root module
        imports.push_str("#[doc(inline)]\n");
        imports.push_str(&format!("pub use crate::bindings::{}::*;\n", header.name));
    }

    // close bindings module
    mods.push_str("}");

    let root_module = format!("{}\n{}", imports, mods);

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
    use crate::HEADER_PREFIX;
    use anyhow::anyhow;
    use bindgen::{builder, Builder};
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::ffi::OsStr;
    use std::fs::{read_to_string, DirEntry};

    enum FilterType {
        Whitelist,
        Blacklist,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum Item {
        Function,
        Item,
        Type,
    }

    #[derive(Deserialize)]
    pub struct HeaderTemplate {
        header: String,
        whitelist: Option<HashMap<String, Item>>,
        blacklist: Option<HashMap<String, Item>>,
    }

    pub struct Header {
        pub name: String,
        header: String,
        filters: Option<(FilterType, HashMap<String, Item>)>,
    }

    impl TryFrom<DirEntry> for Header {
        type Error = anyhow::Error;

        fn try_from(entry: DirEntry) -> Result<Self, Self::Error> {
            let path = entry.path();
            let raw = read_to_string(&path)?;
            let template: HeaderTemplate = toml::from_str(&raw)?;

            let filters = match (template.whitelist, template.blacklist) {
                (None, None) => None,
                (Some(whitelist), None) => Some((FilterType::Whitelist, whitelist)),
                (None, Some(blacklist)) => Some((FilterType::Blacklist, blacklist)),
                // reject the header if it specifies both types of filter
                (Some(_), Some(_)) => {
                    return Err(anyhow!(
                        "found both whitelist and blacklist for {}",
                        path.display()
                    ));
                }
            };

            // set the name of the header to the filename minus the extension
            let name = path
                .file_stem()
                .and_then(OsStr::to_str)
                .map(str::to_string)
                .ok_or_else(|| anyhow!("no valid filename stem for header template found"))?;

            // prefix the header with some global preprocessor directives
            let header = format!("{}\n{}", HEADER_PREFIX, template.header);

            Ok(Header {
                name,
                header,
                filters,
            })
        }
    }

    impl Header {
        /// Create a [`Builder`](bindgen::Builder) from a [`Header`]
        pub fn builder(&self) -> Builder {
            // apply the header to the builder
            let filename = format!("{}.h", &self.name);
            let mut builder = builder().header_contents(&filename, &self.header);

            // apply the found filters
            if let Some((filter_type, filters)) = &self.filters {
                for (name, item) in filters {
                    // set the type of filter to be used on an item
                    let filter = match filter_type {
                        FilterType::Whitelist => match item {
                            Item::Function => Builder::whitelist_function,
                            Item::Type => Builder::whitelist_type,
                            Item::Item => Builder::whitelist_var,
                        },
                        FilterType::Blacklist => match item {
                            Item::Function => Builder::blacklist_function,
                            Item::Type => Builder::blacklist_type,
                            Item::Item => Builder::blacklist_item,
                        },
                    };

                    // apply the filter to the item name
                    builder = filter(builder, name);
                }
            };

            builder
        }
    }
}
