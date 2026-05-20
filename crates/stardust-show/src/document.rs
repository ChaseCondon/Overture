//! Persistence-layer wrapper around `Show`.
//!
//! Per ADR-0003 / ADR-0005, every persisted file carries a header with
//! `kind`, `schema_version`, `stardust_version`, and `saved_at`. The bare
//! `Show` lives at runtime; the document is the on-disk and over-the-wire
//! shape.
//!
//! The `Header` struct is re-used from `stardust-patch` — both document
//! types share the exact same header layout, so a third crate just to hold
//! one struct would be ceremony. See ADR-0005.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use stardust_patch::Header;

use crate::types::Show;

pub const SHOW_KIND: &str = "stardust.show";
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowDocument {
    #[serde(flatten)]
    pub header: Header,
    pub show: Show,
}

impl ShowDocument {
    pub fn new(show: Show) -> Self {
        Self {
            header: Header {
                kind: SHOW_KIND.to_owned(),
                schema_version: CURRENT_SCHEMA_VERSION,
                stardust_version: None,
                saved_at: None,
            },
            show,
        }
    }

    pub fn from_json(s: &str) -> Result<Self, LoadError> {
        let doc: ShowDocument = serde_json::from_str(s)?;
        if doc.header.kind != SHOW_KIND {
            return Err(LoadError::WrongKind {
                expected: SHOW_KIND,
                found: doc.header.kind,
            });
        }
        if doc.header.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(LoadError::NewerSchema {
                document: doc.header.schema_version,
                current: CURRENT_SCHEMA_VERSION,
            });
        }
        // Migration chain goes here once schema_version > 1 exists.
        Ok(doc)
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("not a stardust show document: kind was {found:?}, expected {expected:?}")]
    WrongKind {
        expected: &'static str,
        found: String,
    },

    #[error(
        "show document is schema v{document}, but this build only understands up to v{current}"
    )]
    NewerSchema { document: u32, current: u32 },

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
