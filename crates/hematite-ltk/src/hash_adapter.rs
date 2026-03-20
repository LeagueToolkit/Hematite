//! Hash dictionary adapter — loads from txt files (lmdb later).
//!
//! Implements `HashProvider` from hematite-core.
//!
//! ## Current backend: CDragon txt files
//! Located at `%APPDATA%\RitoShark\Requirements\Hashes\`:
//! - `hashes.bintypes.txt` — class_hash → type name
//! - `hashes.binfields.txt` — name_hash → field name
//! - `hashes.binentries.txt` — path_hash → entry path
//! - `hashes.game.txt` — xxhash64 → asset path
//!
//! ## Future backend: lmdb
//! When lmdb hash loading is ready, create a new `LmdbHashProvider`
//! implementing the same `HashProvider` trait. The CLI switches between
//! backends based on what's available on disk.
//!
//! ## TODO
//! - [ ] Implement TxtHashProvider with pre-computed reverse maps
//! - [ ] Port hash file parsing from old hash_dict.rs
//! - [ ] Build name→hash reverse maps at load time for O(1) lookups
//! - [ ] Add global caching (lazy_static or OnceLock) for single load per process

use std::collections::HashMap;
use hematite_types::hash::{TypeHash, FieldHash};

/// Hash provider backed by CDragon txt files.
pub struct TxtHashProvider {
    /// class_hash → type name
    pub types: HashMap<u32, String>,
    /// field_hash → field name
    pub fields: HashMap<u32, String>,
    /// path_hash → entry path
    pub entries: HashMap<u32, String>,
    /// game_hash → asset path
    pub game_paths: HashMap<u64, String>,

    // Reverse maps (pre-computed at load time)
    /// type name (lowercase) → class_hash
    pub type_name_to_hash: HashMap<String, TypeHash>,
    /// field name (lowercase) → field_hash
    pub field_name_to_hash: HashMap<String, FieldHash>,
}

impl TxtHashProvider {
    /// Load hash dictionaries from the standard RitoShark directory.
    ///
    /// ## TODO
    /// - [ ] Implement file parsing (format: "hash name" per line)
    /// - [ ] Build reverse maps during load
    pub fn load_from_appdata() -> anyhow::Result<Self> {
        // TODO: Read from %APPDATA%\RitoShark\Requirements\Hashes\
        anyhow::bail!("TxtHashProvider::load_from_appdata not yet implemented")
    }
}

// TODO: impl hematite_core::traits::HashProvider for TxtHashProvider
//
// fn resolve_type(&self, hash: TypeHash) -> Option<&str> {
//     self.types.get(&hash.0).map(|s| s.as_str())
// }
// fn resolve_field(&self, hash: FieldHash) -> Option<&str> {
//     self.fields.get(&hash.0).map(|s| s.as_str())
// }
// fn type_hash(&self, name: &str) -> Option<TypeHash> {
//     self.type_name_to_hash.get(&name.to_lowercase()).copied()
// }
// fn field_hash(&self, name: &str) -> Option<FieldHash> {
//     self.field_name_to_hash.get(&name.to_lowercase()).copied()
// }
// ... etc
