//! RegexReplace + RegexRenameField transforms.
//!
//! ## RegexReplace
//! Pattern-based string replacement in string fields.
//! Uses PropertyWalker with `visit_string` for recursive scanning.
//!
//! ## RegexRenameField
//! Regex-based field rename with capture group support.
//! Computes new FNV-1a hash from the renamed field name.
//!
//! ## TODO
//! - [ ] Implement RegexReplacer visitor
//! - [ ] Implement apply_replace() using walk::walk_tree()
//! - [ ] Implement apply_rename() using filter + strings::fnv1a_hash
