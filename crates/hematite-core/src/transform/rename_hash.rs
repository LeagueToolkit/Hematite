//! RenameHash transform.
//!
//! Renames field hashes across the entire BIN tree. Uses PropertyWalker
//! with `visit_field_hash` to find and replace hashes recursively.
//!
//! ## Used by
//! - `staticmat_texturepath`: TextureName → TexturePath
//! - `staticmat_samplername`: SamplerName → TextureName
//!
//! ## Old code: ~90 LOC recursive walk. New code: ~20 LOC visitor impl.

use crate::context::FixContext;
use crate::walk::{walk_tree, PropertyVisitor};
use hematite_types::hash::FieldHash;

struct RenameHashVisitor {
    from: u32,
    to: u32,
}

impl PropertyVisitor for RenameHashVisitor {
    fn visit_field_hash(&mut self, hash: FieldHash) -> Option<FieldHash> {
        if hash.0 == self.from {
            Some(FieldHash(self.to))
        } else {
            None
        }
    }
}

pub fn apply(ctx: &mut FixContext, from_name: &str, to_name: &str) -> u32 {
    let Some(from_hash) = ctx.hashes.field_hash(from_name) else {
        return 0;
    };
    let Some(to_hash) = ctx.hashes.field_hash(to_name) else {
        return 0;
    };

    let mut visitor = RenameHashVisitor {
        from: from_hash.0,
        to: to_hash.0,
    };

    walk_tree(&mut ctx.tree, &mut visitor)
}
