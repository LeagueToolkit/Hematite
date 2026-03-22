//! WAD path lookup and chunk extraction using ltk_wad.

use std::collections::HashSet;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;
use anyhow::{Context, Result};
use hematite_core::traits::{HashProvider, WadProvider};
use hematite_types::hash::GameHash;
use league_toolkit::wad::Wad;
use xxhash_rust::xxh64::xxh64;

/// WAD provider backed by league-toolkit's ltk_wad.
///
/// Stores only the set of path hashes for fast existence checks.
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
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Build from raw WAD bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(data);
        Self::from_reader(cursor)
    }

    /// Internal: Build from any Read+Seek source.
    fn from_reader<R: Read + Seek>(reader: R) -> Result<Self> {
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

/// Opened WAD file with chunk extraction capabilities.
///
/// Wraps the LTK `Wad` handle to support both path lookups (via `build_provider`)
/// and reading individual chunks (for BIN extraction).
pub struct WadFile<R: Read + Seek> {
    wad: Wad<R>,
}

impl WadFile<BufReader<std::fs::File>> {
    /// Open a WAD file from disk.
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open WAD: {:?}", path))?;
        let reader = BufReader::new(file);
        let wad = Wad::mount(reader)
            .map_err(|e| anyhow::anyhow!("Failed to parse WAD: {:?}", e))?;
        Ok(Self { wad })
    }
}

impl<R: Read + Seek> WadFile<R> {
    // SECURITY: Limits to prevent resource exhaustion from malicious WAD files
    const MAX_CHUNK_SIZE: u64 = 100 * 1024 * 1024; // 100MB per chunk
    const MAX_TOTAL_EXTRACTED: u64 = 2 * 1024 * 1024 * 1024; // 2GB total

    /// Build an `LtkWadProvider` from this WAD's chunk list.
    pub fn build_provider(&self) -> LtkWadProvider {
        let mut provider = LtkWadProvider::new();
        for chunk in self.wad.chunks() {
            provider.path_hashes.insert(chunk.path_hash);
        }
        provider
    }

    /// Extract all BIN files from the WAD.
    ///
    /// Uses the hash provider to resolve chunk path hashes to file paths,
    /// then extracts chunks whose path ends with `.bin`.
    /// Returns a vec of (resolved_path, decompressed_bytes) pairs.
    pub fn extract_bin_files(&mut self, hashes: &dyn HashProvider) -> Result<Vec<(String, Vec<u8>)>> {
        // Collect BIN chunk info first (path_hash + resolved path)
        let bin_chunks: Vec<(u64, String)> = self.wad.chunks().iter()
            .filter_map(|chunk| {
                let path = hashes.resolve_game_path(GameHash(chunk.path_hash))?;
                if path.to_lowercase().ends_with(".bin") {
                    Some((chunk.path_hash, path.to_string()))
                } else {
                    None
                }
            })
            .collect();

        let mut results = Vec::with_capacity(bin_chunks.len());
        let mut total_extracted: u64 = 0;

        for (path_hash, path) in bin_chunks {
            let Some(chunk) = self.wad.chunks().get(path_hash) else {
                continue;
            };
            let chunk = *chunk;

            // SECURITY: Check chunk size before extraction
            let chunk_size = chunk.uncompressed_size as u64;
            if chunk_size > Self::MAX_CHUNK_SIZE {
                tracing::warn!(
                    "Skipping large BIN chunk {path}: {} bytes exceeds {} bytes limit",
                    chunk_size,
                    Self::MAX_CHUNK_SIZE
                );
                continue;
            }

            // SECURITY: Check total extracted size
            total_extracted = total_extracted.saturating_add(chunk_size);
            if total_extracted > Self::MAX_TOTAL_EXTRACTED {
                anyhow::bail!(
                    "Total extracted BIN size exceeds limit: {} bytes > {} bytes",
                    total_extracted,
                    Self::MAX_TOTAL_EXTRACTED
                );
            }

            match self.wad.load_chunk_decompressed(&chunk) {
                Ok(data) => {
                    results.push((path, data.to_vec()));
                }
                Err(e) => {
                    tracing::warn!("Failed to extract BIN chunk {path}: {e:?}");
                }
            }
        }

        Ok(results)
    }

    /// Extract ALL files from the WAD with resolved paths.
    ///
    /// Returns a vec of (resolved_path, decompressed_bytes) pairs for all extractable chunks.
    /// Files without resolved paths in the hash dictionary are skipped.
    pub fn extract_all_files(&mut self, hashes: &dyn HashProvider) -> Result<Vec<(String, Vec<u8>)>> {
        // Collect all chunk info (path_hash + resolved path)
        let all_chunks: Vec<(u64, String)> = self.wad.chunks().iter()
            .filter_map(|chunk| {
                let path = hashes.resolve_game_path(GameHash(chunk.path_hash))?;
                Some((chunk.path_hash, path.to_string()))
            })
            .collect();

        let mut results = Vec::with_capacity(all_chunks.len());
        let mut total_extracted: u64 = 0;

        for (path_hash, path) in all_chunks {
            let Some(chunk) = self.wad.chunks().get(path_hash) else {
                continue;
            };
            let chunk = *chunk;

            // SECURITY: Check chunk size before extraction
            let chunk_size = chunk.uncompressed_size as u64;
            if chunk_size > Self::MAX_CHUNK_SIZE {
                tracing::warn!(
                    "Skipping large chunk {path}: {} bytes exceeds {} bytes limit",
                    chunk_size,
                    Self::MAX_CHUNK_SIZE
                );
                continue;
            }

            // SECURITY: Check total extracted size
            total_extracted = total_extracted.saturating_add(chunk_size);
            if total_extracted > Self::MAX_TOTAL_EXTRACTED {
                anyhow::bail!(
                    "Total extracted size from WAD exceeds limit: {} bytes > {} bytes",
                    total_extracted,
                    Self::MAX_TOTAL_EXTRACTED
                );
            }

            match self.wad.load_chunk_decompressed(&chunk) {
                Ok(data) => {
                    results.push((path, data.to_vec()));
                }
                Err(e) => {
                    tracing::debug!("Failed to extract chunk {path}: {e:?}");
                }
            }
        }

        Ok(results)
    }

    /// Extract all BNK files from the WAD.
    ///
    /// Uses the hash provider to resolve chunk path hashes to file paths,
    /// then extracts chunks whose path ends with `.bnk`.
    /// Returns a vec of (resolved_path, decompressed_bytes) pairs.
    pub fn extract_bnk_files(&mut self, hashes: &dyn HashProvider) -> Result<Vec<(String, Vec<u8>)>> {
        // Collect BNK chunk info first (path_hash + resolved path)
        let bnk_chunks: Vec<(u64, String)> = self.wad.chunks().iter()
            .filter_map(|chunk| {
                let path = hashes.resolve_game_path(GameHash(chunk.path_hash))?;
                if path.to_lowercase().ends_with(".bnk") {
                    Some((chunk.path_hash, path.to_string()))
                } else {
                    None
                }
            })
            .collect();

        let mut results = Vec::with_capacity(bnk_chunks.len());
        let mut total_extracted: u64 = 0;

        for (path_hash, path) in bnk_chunks {
            let Some(chunk) = self.wad.chunks().get(path_hash) else {
                continue;
            };
            let chunk = *chunk;

            // SECURITY: Check chunk size before extraction
            let chunk_size = chunk.uncompressed_size as u64;
            if chunk_size > Self::MAX_CHUNK_SIZE {
                tracing::warn!(
                    "Skipping large BNK chunk {path}: {} bytes exceeds {} bytes limit",
                    chunk_size,
                    Self::MAX_CHUNK_SIZE
                );
                continue;
            }

            // SECURITY: Check total extracted size
            total_extracted = total_extracted.saturating_add(chunk_size);
            if total_extracted > Self::MAX_TOTAL_EXTRACTED {
                anyhow::bail!(
                    "Total extracted BNK size exceeds limit: {} bytes > {} bytes",
                    total_extracted,
                    Self::MAX_TOTAL_EXTRACTED
                );
            }

            match self.wad.load_chunk_decompressed(&chunk) {
                Ok(data) => {
                    results.push((path, data.to_vec()));
                }
                Err(e) => {
                    tracing::warn!("Failed to extract BNK chunk {path}: {e:?}");
                }
            }
        }

        Ok(results)
    }
}
