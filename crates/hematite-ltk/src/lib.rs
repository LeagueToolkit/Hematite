//! # hematite-ltk
//!
//! LTK (league-toolkit) adapter crate. **This is the ONLY crate in the workspace
//! that imports `league-toolkit`.** When LTK breaks its public API, only this
//! crate needs updating — the rest of the workspace is unaffected.
//!
//! ## Modules
//! - [`bin_adapter`] — `impl BinProvider` using ltk_meta for BIN parsing/writing
//! - [`hash_adapter`] — `impl HashProvider` for txt file loading (lmdb later)
//! - [`wad_adapter`] — `impl WadProvider` for WAD path lookups via ltk_wad
//! - [`convert`] — Bidirectional conversion: LTK types ↔ Hematite types
//!
//! ## When LTK rewrites land
//! 1. Update `league-toolkit` version in Cargo.toml
//! 2. Fix `convert.rs` — map new LTK types to our PropertyValue enum
//! 3. Fix `bin_adapter.rs` — update parse/write calls
//! 4. Fix `wad_adapter.rs` — update WAD mounting if API changed
//! 5. Everything else stays the same.

pub mod bin_adapter;
pub mod hash_adapter;
pub mod wad_adapter;
pub mod convert;
