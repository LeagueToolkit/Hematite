//! RemoveFromWad transform.
//!
//! Marks the current file for removal from the WAD archive.
//! This is trivial — it just adds the file path to `ctx.files_to_remove`.
//!
//! ## Used by
//! - `champion_bin_remover`: Remove champion data BINs that break mods after patches
//! - `bnk_remover`: Remove BNK audio files with incompatible versions

use crate::context::FixContext;

/// Mark the current file for removal from the WAD.
/// Returns 1 (one removal action).
pub fn apply(ctx: &mut FixContext<'_>) -> u32 {
    ctx.files_to_remove.push(ctx.file_path.clone());
    1
}
