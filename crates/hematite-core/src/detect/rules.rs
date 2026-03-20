//! Detection rule dispatch and individual rule implementations.
//!
//! ## Detection rules and their logic
//!
//! | Rule | What it checks |
//! |------|---------------|
//! | `MissingOrWrongField` | Field missing or has wrong value in embed path |
//! | `FieldHashExists` | A field hash exists at a dot-separated path |
//! | `StringExtensionNotInWad` | String fields with extension not in WAD |
//! | `RecursiveStringExtensionNotInWad` | Recursive scan for extension strings not in WAD |
//! | `EntryTypeExistsAny` | Any object matches entry type list |
//! | `BnkVersionNotIn` | BNK audio version not in allowed list |
//! | `VfxShapeNeedsFix` | VFX shape has old format (pre-14.1) |
//!
//! ## Shared utilities used
//! - [`crate::filter`] — `objects_by_type`, `has_any_type`
//! - [`crate::walk`] — `extract_strings` for recursive string scanning
//! - [`crate::factory`] — `matches_json` for value comparison
//!
//! ## TODO
//! - [ ] Implement detect_issue() dispatch
//! - [ ] Implement each detection rule function
//! - [ ] Port search_field_path() logic using PropertyWalker
//! - [ ] Handle BnkVersionNotIn (operates on raw bytes, not BIN tree)

use hematite_types::bin::BinTree;
use hematite_types::config::DetectionRule;
use crate::traits::{HashProvider, WadProvider};

/// Main detection dispatch. Returns true if the issue is detected.
///
/// ## TODO
/// - [ ] Implement match on all DetectionRule variants
pub fn detect_issue(
    _rule: &DetectionRule,
    _tree: &BinTree,
    _hashes: &dyn HashProvider,
    _wad: &dyn WadProvider,
) -> bool {
    // TODO: Match on rule variant, delegate to specific detection function
    //
    // match rule {
    //     DetectionRule::MissingOrWrongField { .. } => detect_missing_or_wrong_field(..),
    //     DetectionRule::FieldHashExists { .. } => detect_field_hash_exists(..),
    //     DetectionRule::StringExtensionNotInWad { .. } => detect_string_ext_not_in_wad(..),
    //     DetectionRule::RecursiveStringExtensionNotInWad { .. } => detect_recursive_ext(..),
    //     DetectionRule::EntryTypeExistsAny { .. } => detect_entry_type_exists(..),
    //     DetectionRule::BnkVersionNotIn { .. } => detect_bnk_version(..),
    //     DetectionRule::VfxShapeNeedsFix { .. } => detect_vfx_shape(..),
    // }
    false
}

// TODO: Individual detection functions below
//
// fn detect_missing_or_wrong_field(..) -> bool { }
// fn detect_field_hash_exists(..) -> bool { }
// fn detect_string_ext_not_in_wad(..) -> bool { }
// fn detect_recursive_ext(..) -> bool { }
// fn detect_entry_type_exists(..) -> bool { }
// fn detect_bnk_version(..) -> bool { }
// fn detect_vfx_shape(..) -> bool { }
