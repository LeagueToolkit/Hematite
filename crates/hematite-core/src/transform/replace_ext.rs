//! ReplaceStringExtension transform.
//!
//! Replaces file extensions in all string values (e.g. .dds → .tex, .tex → .dds).
//! Only replaces if the target file doesn't exist in the WAD (prevents breaking
//! existing references).
//!
//! ## Used by
//! - `black_icons`: .dds → .tex (fixes missing icon textures)
//! - `dds_to_tex`: .dds → .tex (standardizes texture format)
//!
//! ## Old code: ~100 LOC recursive walk. New code: ~25 LOC visitor impl.

use crate::context::FixContext;
use crate::strings::replace_extension;
use crate::walk::{walk_tree, PropertyVisitor, VisitResult};
use hematite_types::hash::FieldHash;

struct ExtensionReplacer<'a> {
    from: &'a str,
    to: &'a str,
    wad: &'a dyn crate::traits::WadProvider,
}

impl PropertyVisitor for ExtensionReplacer<'_> {
    fn visit_string(&mut self, value: &str, _hash: FieldHash) -> VisitResult {
        if value.to_lowercase().ends_with(self.from) && !self.wad.has_path(value) {
            if let Some(new_val) = replace_extension(value, self.from, self.to) {
                VisitResult::Mutate(new_val)
            } else {
                VisitResult::Skip
            }
        } else {
            VisitResult::Skip
        }
    }
}

pub fn apply(ctx: &mut FixContext, from: &str, to: &str) -> u32 {
    let mut visitor = ExtensionReplacer {
        from,
        to,
        wad: ctx.wad,
    };

    walk_tree(&mut ctx.tree, &mut visitor)
}
