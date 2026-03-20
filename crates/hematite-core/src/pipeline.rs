//! Fix orchestration: detect → transform → result.
//!
//! This is the main entry point for the fix engine. Given a `FixContext` and
//! a set of selected fix rules, it:
//! 1. Runs detection for each rule
//! 2. If detected, applies the corresponding transform
//! 3. Collects results (applied fixes, failures, change counts)
//!
//! ## Flow
//! ```text
//! for each fix_id in selected_fixes:
//!     rule = config.fixes[fix_id]
//!     if detect::detect_issue(&rule.detect, &ctx):
//!         changes = transform::apply_transform(&rule.apply, &mut ctx)
//!         track result
//! ```
//!
//! ## TODO
//! - [ ] Implement apply_fixes() orchestration function
//! - [ ] Handle detection-before-transform pattern (verify issue still exists)
//! - [ ] Track per-fix results in ProcessResult
//! - [ ] Support dry-run mode (detect only, no transforms)

use hematite_types::config::FixConfig;
use hematite_types::result::ProcessResult;
use crate::context::FixContext;

/// Run selected fixes against a BIN tree.
///
/// Returns the modified BinTree (inside the context) and a result summary.
///
/// ## TODO
/// - [ ] Implement this
pub fn apply_fixes(
    _ctx: &mut FixContext<'_>,
    _config: &FixConfig,
    _selected_fix_ids: &[String],
    _dry_run: bool,
) -> ProcessResult {
    // TODO: Implement fix orchestration
    // 1. For each fix_id, look up the rule in config
    // 2. Run detect::detect_issue() with the detection rule
    // 3. If detected and not dry_run, run transform::apply_transform()
    // 4. Track results
    ProcessResult::default()
}
