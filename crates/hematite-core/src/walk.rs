//! PropertyWalker — single recursive traversal engine.
//!
//! **This replaces 6 separate recursive walk implementations** from the old codebase:
//! - `bin_parser.rs::extract_strings_from_value`
//! - `applier.rs::rename_hash_in_value`
//! - `applier.rs::replace_extension_in_value`
//! - `applier.rs::regex_replace_in_value`
//! - `detector.rs::search_field_in_value`
//! - `detector.rs::search_field_path`
//!
//! Instead of each fix module writing its own recursive match over PropertyValue,
//! they implement the [`PropertyVisitor`] trait and let the walker handle traversal.
//!
//! ## Example usage (replacing ~90 LOC with ~15 LOC)
//! ```ignore
//! struct ExtReplacer<'a> { from: &'a str, to: &'a str, wad: &'a dyn WadProvider }
//!
//! impl PropertyVisitor for ExtReplacer<'_> {
//!     fn visit_string(&mut self, value: &str, _hash: FieldHash) -> VisitResult {
//!         if value.to_lowercase().ends_with(self.from) && !self.wad.has_path(value) {
//!             VisitResult::Mutate(strings::replace_extension(value, self.from, self.to).unwrap())
//!         } else {
//!             VisitResult::Skip
//!         }
//!     }
//! }
//!
//! let changes = walk::walk_object(&mut obj, &mut ExtReplacer { from, to, wad });
//! ```
//!
//! ## TODO
//! - [ ] Implement walk_properties() recursive traversal
//! - [ ] Handle all PropertyValue variants: String, Container,
//!   UnorderedContainer, Embedded, Struct, Optional, Map
//! - [ ] Ensure mutable visitor can rename field hashes (keys in IndexMap)
//! - [ ] Add walk_tree() convenience for walking all objects

use hematite_types::bin::{BinTree, BinObject};
use hematite_types::hash::FieldHash;

/// Result of visiting a string value.
pub enum VisitResult {
    /// Don't change anything.
    Skip,
    /// Replace the string with a new value.
    Mutate(String),
}

/// Visitor trait for property tree traversal.
///
/// Implement only the methods you need — defaults are no-ops.
/// The walker calls these as it recurses through the property tree.
#[allow(unused_variables)]
pub trait PropertyVisitor {
    /// Called for each string value found.
    /// Return `VisitResult::Mutate(new)` to replace the string.
    fn visit_string(&mut self, value: &str, field_hash: FieldHash) -> VisitResult {
        VisitResult::Skip
    }

    /// Called for each field hash encountered.
    /// Return `Some(new_hash)` to rename the field.
    fn visit_field_hash(&mut self, hash: FieldHash) -> Option<FieldHash> {
        None
    }

    /// Called when entering an embedded/struct.
    /// Return false to skip its children.
    fn enter_struct(&mut self, class_hash: u32) -> bool {
        true
    }
}

/// Walk all properties in a BinObject, calling visitor methods.
/// Returns the number of mutations applied.
///
/// ## TODO
/// - [ ] Implement recursive traversal over all PropertyValue variants
pub fn walk_object(_obj: &mut BinObject, _visitor: &mut dyn PropertyVisitor) -> u32 {
    // TODO: Iterate obj.properties, for each call walk_property()
    // which recurses into Embedded/Struct/Container/Optional/Map
    0
}

/// Walk all objects in a BinTree.
/// Returns the total number of mutations applied.
pub fn walk_tree(tree: &mut BinTree, visitor: &mut dyn PropertyVisitor) -> u32 {
    tree.objects
        .values_mut()
        .map(|obj| walk_object(obj, visitor))
        .sum()
}

/// Extract all string values from a BinTree (read-only).
///
/// Replaces `bin_parser.rs::extract_all_strings()` from the old codebase.
///
/// ## TODO
/// - [ ] Implement using a read-only visitor or direct recursion
pub fn extract_strings(_tree: &BinTree) -> Vec<String> {
    // TODO: Walk all properties, collect strings
    Vec::new()
}
