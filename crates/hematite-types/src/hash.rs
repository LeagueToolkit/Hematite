//! Newtype wrappers for League's hash kinds.
//!
//! League uses hashed identifiers instead of strings for performance.
//! These newtypes prevent mixing up hash kinds at compile time.
//!
//! | Kind | Width | Usage |
//! |------|-------|-------|
//! | [`TypeHash`] | u32 | `class_hash` — identifies BIN object type (e.g. "SkinCharacterDataProperties") |
//! | [`FieldHash`] | u32 | `name_hash` — identifies field name (e.g. "UnitHealthBarStyle") |
//! | [`PathHash`] | u32 | `path_hash` — identifies entry path in BIN |
//! | [`GameHash`] | u64 | xxhash64 — identifies asset paths in WAD files |

use serde::{Deserialize, Serialize};

/// BIN object class hash (u32, FNV-1a of type name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeHash(pub u32);

/// BIN property field name hash (u32, FNV-1a of field name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldHash(pub u32);

/// BIN entry path hash (u32, FNV-1a of entry path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathHash(pub u32);

/// WAD asset path hash (u64, xxhash64 of lowercased asset path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameHash(pub u64);
