//! Bidirectional conversion between LTK types and Hematite types.
//!
//! This is the critical module that isolates the rest of the codebase from
//! LTK's type system. When LTK rewrites its `PropertyValueEnum`, only this
//! file needs updating.
//!
//! ## Conversion paths
//! - `ltk_tree_to_hematite()` — LTK Bin → Hematite BinTree (after parsing)
//! - `hematite_tree_to_ltk()` — Hematite BinTree → LTK Bin (before writing)
//! - `ltk_value_to_hematite()` — single PropertyValueEnum → PropertyValue
//! - `hematite_value_to_ltk()` — single PropertyValue → PropertyValueEnum
//!
//! ## LTK 0.4+ specifics (current)
//! - Container is enum with variants: `Container::String { items }`, etc.
//! - Optional is enum: `Optional::String(Some(inner))`, etc.
//! - String fields use `.value` instead of `.0`
//! - Embedded wraps Struct: `Embedded(Struct { ... })`
//!
//! ## When LTK rewrites
//! Update the match arms in ltk_value_to_hematite / hematite_value_to_ltk
//! to handle the new variant shapes. The Hematite PropertyValue enum stays stable.
//!
//! ## TODO
//! - [ ] Implement ltk_tree_to_hematite (full tree conversion)
//! - [ ] Implement hematite_tree_to_ltk (full tree conversion)
//! - [ ] Implement ltk_value_to_hematite for ALL PropertyValueEnum variants
//! - [ ] Implement hematite_value_to_ltk for ALL PropertyValue variants
//! - [ ] Add round-trip tests to verify no data loss
