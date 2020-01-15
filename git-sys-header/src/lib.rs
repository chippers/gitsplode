mod error;
mod filter;
mod item;

pub use error::Error;

use crate::filter::{Filter, FilterType};
use bindgen::Builder;
use serde::Deserialize;
use std::io::Read;

/// Representation of a header to import and generate bindings to
#[derive(Deserialize)]
pub struct Header {
    header: String,
    whitelist: Option<Vec<Filter>>,
    blacklist: Option<Vec<Filter>>,
}

impl Header {
    pub fn new(mut reader: impl Read, prefix: impl AsRef<str>) -> Result<Self, Error> {
        let mut raw = String::new();
        reader.read_to_string(&mut raw)?;
        let mut header: Self = toml::from_str(&raw)?;

        // reject the header if it specifies both types of filter
        if header.whitelist.is_some() && header.blacklist.is_some() {
            return Err(Error::DualFilters);
        }

        // add the prefix to the header
        header.header = format!("{}\n{}", prefix.as_ref(), header.header);

        Ok(header)
    }

    pub fn into_builder(self, name: &str) -> Builder {
        let mut builder = Builder::default();

        builder = builder.header_contents(name, &self.header);

        let filters = self
            .whitelist
            .map(|f| (f, FilterType::Whitelist))
            .or(self.blacklist.map(|f| (f, FilterType::Blacklist)));

        if let Some((filters, filter_type)) = filters {
            for filter in filters {
                builder = filter.build(builder, filter_type)
            }
        };

        builder
    }
}
