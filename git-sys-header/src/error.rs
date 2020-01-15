/// All errors that can happen while parsing headers
pub enum Error {
    /// Any error from `toml`
    Deserialize(toml::de::Error),
    /// Both a Whitelist AND a Blacklist is specified
    DualFilters,
    /// Invalid filename passed in path
    Filename,
    /// Cannot read valid input from the source
    Read(std::io::Error),
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::Deserialize(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Read(err)
    }
}
