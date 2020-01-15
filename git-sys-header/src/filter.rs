use crate::item::Item;
use bindgen::Builder;
use serde::Deserialize;

/// An item from a header, useful for whitelisting or blacklisting
#[derive(Deserialize)]
pub(crate) struct Filter {
    /// The type of item to filter
    item: Item,
    /// The ident of the item to filter
    name: String,
}

#[derive(Copy, Clone)]
pub(crate) enum FilterType {
    Whitelist,
    Blacklist,
}

impl Filter {
    pub fn build(self, builder: Builder, r#type: FilterType) -> Builder {
        let filter = match r#type {
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

        filter(builder, self.name)
    }
}
