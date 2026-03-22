//! Fix context — runtime state for a fix session.
//!
//! `FixContext` bundles together everything a detection rule or transform action
//! needs: the BIN tree being processed, hash lookups, WAD existence checks,
//! and champion relationship data.

use crate::traits::{HashProvider, WadProvider};
use hematite_types::bin::BinTree;
use hematite_types::champion::CharacterRelations;

/// Runtime state for a fix session on a single BIN file.
///
/// Passed to detection rules and transform actions. The BIN tree is mutable
/// so transforms can modify it in-place.
pub struct FixContext<'a> {
    /// The BIN tree being processed (mutable for transforms).
    pub tree: BinTree,

    /// Hash dictionary for name ↔ hash resolution.
    pub hashes: &'a dyn HashProvider,

    /// WAD cache for asset existence checks.
    pub wad: &'a dyn WadProvider,

    /// Champion → subchamp relationships.
    pub champions: &'a CharacterRelations,

    /// Path of the current file being processed (for logging/context).
    pub file_path: String,

    /// Files marked for removal from the WAD (populated by RemoveFromWad transforms).
    pub files_to_remove: Vec<String>,
}
