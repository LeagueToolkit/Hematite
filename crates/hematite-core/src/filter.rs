//! ObjectFilter — iterate BIN objects by type hash.
//!
//! **Replaces 15+ inline iteration loops** like:
//! ```ignore
//! for (_path_hash, obj) in &tree.objects {
//!     if obj.class_hash != target { continue; }
//!     // ...
//! }
//! ```
//!
//! Now:
//! ```ignore
//! for obj in filter::objects_by_type(&tree, type_hash) {
//!     // ...
//! }
//! ```
//!
//! ## TODO
//! - [ ] Consider adding filter by multiple types (for EntryTypeExistsAny)

use hematite_types::bin::{BinTree, BinObject};
use hematite_types::hash::TypeHash;

/// Iterate objects matching a specific class hash (immutable).
pub fn objects_by_type(tree: &BinTree, class_hash: TypeHash) -> impl Iterator<Item = &BinObject> {
    tree.objects.values().filter(move |obj| obj.class_hash == class_hash)
}

/// Get path_hash keys of objects matching a class hash.
///
/// Used for mutable access patterns where you need to iterate keys
/// then get `&mut` references (Rust borrow checker workaround).
pub fn object_keys_by_type(tree: &BinTree, class_hash: TypeHash) -> Vec<u32> {
    tree.objects
        .iter()
        .filter(|(_, obj)| obj.class_hash == class_hash)
        .map(|(k, _)| *k)
        .collect()
}

/// Check if any object in the tree matches one of the given class hashes.
///
/// Used by `EntryTypeExistsAny` detection rule.
pub fn has_any_type(tree: &BinTree, class_hashes: &[TypeHash]) -> bool {
    tree.objects.values().any(|obj| class_hashes.contains(&obj.class_hash))
}
