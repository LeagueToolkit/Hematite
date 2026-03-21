//! BIN parsing adapter using ltk_meta.

use std::io::Cursor;
use anyhow::{Result, bail};
use hematite_types::bin::BinTree;
use hematite_core::traits::BinProvider;
use league_toolkit::meta::Bin as LtkBin;
use crate::convert::ltk_tree_to_hematite;

/// BIN provider backed by league-toolkit's ltk_meta.
pub struct LtkBinProvider;

impl LtkBinProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LtkBinProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BinProvider for LtkBinProvider {
    fn parse_bytes(&self, data: &[u8]) -> Result<BinTree> {
        let mut cursor = Cursor::new(data);
        let ltk_tree = LtkBin::from_reader(&mut cursor)
            .map_err(|e| anyhow::anyhow!("Failed to parse BIN: {:?}", e))?;
        ltk_tree_to_hematite(ltk_tree)
    }

    fn write_bytes(&self, _tree: &BinTree) -> Result<Vec<u8>> {
        // TODO: LTK Bin has private fields preventing reconstruction
        bail!("BIN writing not yet implemented")
    }
}
