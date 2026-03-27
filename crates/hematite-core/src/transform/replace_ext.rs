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
    path_prefixes: &'a [String],
}

impl PropertyVisitor for ExtensionReplacer<'_> {
    fn visit_string(&mut self, value: &str, _hash: FieldHash) -> VisitResult {
        let lower = value.to_lowercase();

        // Check extension
        if !lower.ends_with(self.from) {
            return VisitResult::Skip;
        }

        // CRITICAL: Check WAD FIRST - if file exists in WAD, NEVER convert (custom file included)
        // This handles repathed mods correctly - they can put files anywhere they want
        if self.wad.has_path(value) {
            return VisitResult::Skip;
        }

        // File NOT in WAD - check if it's an official game path that needs conversion
        // Skip custom paths (not official game assets) - those are broken mods, not our problem
        if !self.path_prefixes.is_empty() {
            let matches_prefix = self
                .path_prefixes
                .iter()
                .any(|prefix| lower.starts_with(&prefix.to_lowercase()));
            if !matches_prefix {
                return VisitResult::Skip;
            }
        }

        // All checks passed: .dds extension + NOT in WAD + official path → convert
        if let Some(new_val) = replace_extension(value, self.from, self.to) {
            VisitResult::Mutate(new_val)
        } else {
            VisitResult::Skip
        }
    }
}

pub fn apply(ctx: &mut FixContext, from: &str, to: &str, path_prefixes: &[String]) -> u32 {
    let mut visitor = ExtensionReplacer {
        from,
        to,
        wad: ctx.wad,
        path_prefixes,
    };

    walk_tree(&mut ctx.tree, &mut visitor)
}
