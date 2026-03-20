//! BIN file parsing/writing adapter using ltk_meta.
//!
//! Implements `BinProvider` from hematite-core, converting between
//! LTK's `BinTree` and Hematite's `BinTree`.
//!
//! ## LTK types used
//! - `league_toolkit::meta::Bin` (our BinTree)
//! - `league_toolkit::meta::BinObject`
//! - `league_toolkit::meta::PropertyValueEnum`
//!
//! ## TODO
//! - [ ] Implement BinProvider::parse_bytes using Bin::from_reader()
//! - [ ] Implement BinProvider::write_bytes using Bin::to_writer()
//! - [ ] Use convert module for LTK ↔ Hematite type mapping

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

// TODO: impl hematite_core::traits::BinProvider for LtkBinProvider
//
// fn parse_bytes(&self, data: &[u8]) -> Result<BinTree> {
//     let ltk_tree = league_toolkit::meta::Bin::from_reader(&mut Cursor::new(data))?;
//     convert::ltk_tree_to_hematite(ltk_tree)
// }
//
// fn write_bytes(&self, tree: &BinTree) -> Result<Vec<u8>> {
//     let ltk_tree = convert::hematite_tree_to_ltk(tree)?;
//     let mut buf = Vec::new();
//     ltk_tree.to_writer(&mut buf)?;
//     Ok(buf)
// }
