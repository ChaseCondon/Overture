//! Persistence-layer wrapper around `PatchGraph`.
//!
//! Per ADR-0003, every persisted file carries a header with `kind`,
//! `schema_version`, `stardust_version`, and `saved_at`. The bare graph
//! lives at runtime; the document is the on-disk and over-the-wire shape.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::PatchGraph;

pub const PATCH_KIND: &str = "stardust.patch";
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub kind: String,
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stardust_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub saved_at: Option<String>,
}

impl Header {
    pub fn current() -> Self {
        Self {
            kind: PATCH_KIND.to_owned(),
            schema_version: CURRENT_SCHEMA_VERSION,
            stardust_version: None,
            saved_at: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchDocument {
    #[serde(flatten)]
    pub header: Header,
    pub graph: PatchGraph,
}

impl PatchDocument {
    pub fn new(graph: PatchGraph) -> Self {
        Self {
            header: Header::current(),
            graph,
        }
    }

    pub fn from_json(s: &str) -> Result<Self, LoadError> {
        let doc: PatchDocument = serde_json::from_str(s)?;
        if doc.header.kind != PATCH_KIND {
            return Err(LoadError::WrongKind {
                expected: PATCH_KIND,
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
    #[error("not a stardust patch document: kind was {found:?}, expected {expected:?}")]
    WrongKind {
        expected: &'static str,
        found: String,
    },

    #[error(
        "patch document is schema v{document}, but this build only understands up to v{current}"
    )]
    NewerSchema { document: u32, current: u32 },

    #[error("malformed patch JSON: {0}")]
    Json(#[from] serde_json::Error),
}
