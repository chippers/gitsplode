use serde::Deserialize;

/// Available items to target from a header
#[derive(Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub(crate) enum Item {
    Function,
    Item,
    Type,
}
