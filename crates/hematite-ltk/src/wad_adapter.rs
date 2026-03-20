//! WAD path lookup adapter using ltk_wad.
//!
//! Implements `WadProvider` from hematite-core.
//! Builds an in-memory index of xxhash64 path hashes from WAD chunks.
//!
//! ## LTK types used
//! - `league_toolkit::wad::Wad`
//! - `league_toolkit::wad::WadChunk`
//!
//! ## TODO
//! - [ ] Implement LtkWadProvider with hash set of path hashes
//! - [ ] Port WAD mounting and chunk indexing from old wad_cache.rs
//! - [ ] Add xxhash64 computation for has_path()

use std::collections::HashSet;

/// WAD provider backed by league-toolkit's ltk_wad.
pub struct LtkWadProvider {
    /// Set of xxhash64 path hashes present in the WAD.
    #[allow(dead_code)]
    path_hashes: HashSet<u64>,
}

impl LtkWadProvider {
    /// Build from a WAD file on disk.
    ///
    /// ## TODO
    /// - [ ] Implement using Wad::mount() and iterate chunks
    pub fn from_file(_path: &std::path::Path) -> anyhow::Result<Self> {
        // TODO: Mount WAD, iterate chunks, collect path hashes
        anyhow::bail!("LtkWadProvider::from_file not yet implemented")
    }

    /// Build from raw WAD bytes.
    ///
    /// ## TODO
    /// - [ ] Implement using Wad::from_reader()
    pub fn from_bytes(_data: &[u8]) -> anyhow::Result<Self> {
        anyhow::bail!("LtkWadProvider::from_bytes not yet implemented")
    }
}

// TODO: impl hematite_core::traits::WadProvider for LtkWadProvider
//
// fn has_path(&self, path: &str) -> bool {
//     let normalized = hematite_core::strings::normalize_wad_path(path);
//     let hash = xxhash_rust::xxh64::xxh64(normalized.as_bytes(), 0);
//     self.path_hashes.contains(&hash)
// }
//
// fn has_hash(&self, hash: u64) -> bool {
//     self.path_hashes.contains(&hash)
// }
