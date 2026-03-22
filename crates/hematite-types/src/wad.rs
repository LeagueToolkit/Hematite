//! WAD chunk metadata and modification tracking.
//!
//! WAD files are League's archive format containing BIN, DDS, SKN, and other assets.
//! This module defines lightweight types for tracking which chunks exist and
//! which modifications to apply during repacking.

use crate::hash::GameHash;

/// Metadata for a single chunk in a WAD file.
#[derive(Debug, Clone)]
pub struct WadChunkInfo {
    /// xxhash64 of the asset path.
    pub path_hash: GameHash,
    /// Compressed size in bytes.
    pub compressed_size: u32,
    /// Uncompressed size in bytes.
    pub uncompressed_size: u32,
}

/// Tracks how a WAD chunk should be handled during rebuild.
#[derive(Debug, Clone)]
pub enum WadModification {
    /// Keep original chunk unchanged.
    Original,
    /// Replace chunk data with new bytes.
    Modified(Vec<u8>),
    /// Remove chunk from output WAD.
    Removed,
}
