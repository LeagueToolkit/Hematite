//! WAD path lookup using ltk_wad.

use std::collections::HashSet;
use std::io::{Cursor, Read};
use std::path::Path;
use anyhow::{Context, Result};
use hematite_core::traits::WadProvider;
use league_toolkit::wad::Wad;
use xxhash_rust::xxh64::xxh64;

/// WAD provider backed by league-toolkit's ltk_wad.
pub struct LtkWadProvider {
    /// Set of xxhash64 path hashes present in the WAD.
    path_hashes: HashSet<u64>,
}

impl LtkWadProvider {
    /// Create empty WAD provider.
    pub fn new() -> Self {
        Self {
            path_hashes: HashSet::new(),
        }
    }

    /// Build from a WAD file on disk.
    pub fn from_file(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open WAD: {:?}", path))?;
        let reader = std::io::BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Build from raw WAD bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(data);
        Self::from_reader(cursor)
    }

    /// Internal: Build from any Read+Seek source.
    fn from_reader<R: Read + std::io::Seek>(reader: R) -> Result<Self> {
        let wad = Wad::mount(reader)
            .map_err(|e| anyhow::anyhow!("Failed to parse WAD: {:?}", e))?;

        let mut provider = Self::new();

        for chunk in wad.chunks() {
            provider.path_hashes.insert(chunk.path_hash);
        }

        Ok(provider)
    }

    /// Get total hash count.
    pub fn hash_count(&self) -> usize {
        self.path_hashes.len()
    }
}

impl Default for LtkWadProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl WadProvider for LtkWadProvider {
    fn has_path(&self, path: &str) -> bool {
        let normalized = path.to_lowercase().replace('\\', "/");
        let hash = xxh64(normalized.as_bytes(), 0);
        self.path_hashes.contains(&hash)
    }

    fn has_hash(&self, hash: u64) -> bool {
        self.path_hashes.contains(&hash)
    }
}
