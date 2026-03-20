//! Fix transform actions.
//!
//! Each [`TransformAction`] variant maps to a handler in its own module.
//! Transforms mutate the BIN tree in-place and return a change count.
//!
//! ## Modules and what they handle
//! | Module | TransformAction | Shared utils |
//! |--------|----------------|--------------|
//! | [`ensure_field`] | `EnsureField`, `EnsureFieldWithContext` | ObjectFilter, ValueFactory |
//! | [`rename_hash`] | `RenameHash` | PropertyWalker (visit_field_hash) |
//! | [`replace_ext`] | `ReplaceStringExtension` | PropertyWalker (visit_string) |
//! | [`change_type`] | `ChangeFieldType` | ObjectFilter, ValueFactory |
//! | [`regex_ops`] | `RegexReplace`, `RegexRenameField` | PropertyWalker (visit_string) |
//! | [`vfx_shape`] | `VfxShapeFix` | ObjectFilter |
//! | [`remove`] | `RemoveFromWad` | (trivial) |

pub mod ensure_field;
pub mod rename_hash;
pub mod replace_ext;
pub mod change_type;
pub mod regex_ops;
pub mod vfx_shape;
pub mod remove;

use hematite_types::config::TransformAction;
use crate::context::FixContext;

/// Main transform dispatch. Returns number of changes applied.
///
/// ## TODO
/// - [ ] Implement match on all TransformAction variants
pub fn apply_transform(
    _action: &TransformAction,
    _ctx: &mut FixContext<'_>,
) -> u32 {
    // TODO: Match on action variant, delegate to specific transform function
    //
    // match action {
    //     TransformAction::EnsureField { .. } => ensure_field::apply(..),
    //     TransformAction::RenameHash { .. } => rename_hash::apply(..),
    //     TransformAction::ReplaceStringExtension { .. } => replace_ext::apply(..),
    //     TransformAction::RemoveFromWad => remove::apply(..),
    //     TransformAction::ChangeFieldType { .. } => change_type::apply(..),
    //     TransformAction::RegexReplace { .. } => regex_ops::apply_replace(..),
    //     TransformAction::RegexRenameField { .. } => regex_ops::apply_rename(..),
    //     TransformAction::VfxShapeFix => vfx_shape::apply(..),
    // }
    0
}
