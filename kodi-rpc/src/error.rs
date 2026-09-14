use std::{error::Error, fmt::Display};

/// Error type
#[derive(Debug)]
pub enum KodiError {
    /// MediaType is None / nothing playable.
    /// NOTE: `Unknown` (3rd-party plugin playback, `type: "unknown"` with a
    /// `plugin://` file) is intentionally *not* an error and will display.
    UnrecognizedMediaType,
    /// Content is in blacklist
    ContentBlacklist,
    MissingRequiredValues,
    NoImage,
    /// No active Kodi player (nothing playing). Treated like "clear activity".
    NothingPlaying,
}

impl Error for KodiError {}

impl Display for KodiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KodiError::MissingRequiredValues => {
                write!(f, "missing required values to build client")
            }
            KodiError::UnrecognizedMediaType => write!(f, "unrecognized media type"),
            KodiError::ContentBlacklist => write!(f, "content is blacklisted"),
            KodiError::NoImage => write!(f, "media does not have an image"),
            KodiError::NothingPlaying => write!(f, "nothing is playing"),
        }
    }
}
