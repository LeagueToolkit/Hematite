//! ReplaceStringExtension transform.
//!
//! Replaces file extensions in all string values (e.g. `.dds` → `.tex`).
//! Only replaces if the original path does NOT exist in the WAD cache
//! (prevents fixing files that aren't actually broken).
//!
//! Uses PropertyWalker with `visit_string` for recursive string scanning.
//!
//! ## Used by
//! - `black_icons`: Icon .dds → .tex conversion
//! - `dds_to_tex`: All texture .dds → .tex conversion
//!
//! ## Old code: ~90 LOC recursive walk. New code: ~20 LOC visitor impl.
//!
//! ## TODO
//! - [ ] Implement ExtensionReplacer visitor
//! - [ ] Implement apply() using walk::walk_tree()
//! - [ ] Add WAD existence check in visit_string
