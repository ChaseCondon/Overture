//! Core show types: rig, songs, patches, saved blocks, the show.
//!
//! Mirrors the TS shapes in `stardust-pit/src/src/screens/_seed-data.ts`
//! and adjacent component files. Field names are camelCase on the wire so
//! the TS UI can produce and consume `ShowDocument` JSON without adapters.

use serde::{Deserialize, Serialize};

use stardust_patch::{NodeKind, PatchGraph};

// -----------------------------------------------------------------------------
// ID newtypes (same pattern as stardust-patch)
// -----------------------------------------------------------------------------

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

string_id!(SongId);
string_id!(PatchId);
string_id!(BlockId);

// -----------------------------------------------------------------------------
// Rig
// -----------------------------------------------------------------------------

/// One physical input the user has configured in their rig. The `kind` maps
/// to a `source.*` `NodeKind`; the `label` is the user's friendly name
/// ("Nord Stage 3 keys"). Two rig sources of the same kind with different
/// labels is valid — someone with two keyboards has two `source.keyboard`
/// entries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RigSource {
    pub kind: NodeKind,
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rig {
    pub sources: Vec<RigSource>,
}

// -----------------------------------------------------------------------------
// Saved blocks (user-created composite presets)
// -----------------------------------------------------------------------------

/// A user-saved composite block, shown in the right-panel Blocks tab. v1
/// stores only metadata — the actual subgraph for re-instantiation is a
/// future revisit (see ADR-0005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedBlock {
    pub id: BlockId,
    pub name: String,
    pub node_count: u32,
}

// -----------------------------------------------------------------------------
// Songs + patches
// -----------------------------------------------------------------------------

/// One patch within a song. Inlines its `PatchGraph` directly — see
/// ADR-0005 for the inline-vs-side-table decision. `compound` is a v1
/// placeholder for multi-part patches (verse/chorus/bridge); structural
/// support for parts is deferred.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patch {
    pub id: PatchId,
    pub number: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub compound: bool,
    pub graph: PatchGraph,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: SongId,
    pub number: u32,
    pub name: String,
    pub patches: Vec<Patch>,
}

// -----------------------------------------------------------------------------
// The show
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub name: String,
    pub songs: Vec<Song>,
    pub rig: Rig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub saved_blocks: Vec<SavedBlock>,
}

impl Show {
    pub fn find_song(&self, id: &SongId) -> Option<&Song> {
        self.songs.iter().find(|s| &s.id == id)
    }

    pub fn find_patch(&self, id: &PatchId) -> Option<(&Song, &Patch)> {
        for s in &self.songs {
            if let Some(p) = s.patches.iter().find(|p| &p.id == id) {
                return Some((s, p));
            }
        }
        None
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}
