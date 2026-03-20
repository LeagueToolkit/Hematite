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
//!
//! ## TODO
//! - [ ] Implement RenameHashVisitor
//! - [ ] Implement apply() using walk::walk_tree()
