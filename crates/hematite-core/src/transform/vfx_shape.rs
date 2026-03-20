//! VfxShapeFix transform.
//!
//! Complex VFX shape structure migration for post-patch 14.1 changes.
//! This is the most complex transform — it restructures VFX shape embeds
//! by moving BirthTranslation out of Shape and converting field formats.
//!
//! This transform is bespoke and doesn't benefit much from shared utilities
//! beyond ObjectFilter for initial matching.
//!
//! ## What it does
//! 1. Find `VfxSystemDefinitionData` objects
//! 2. Look for Shape embeds with old-format fields
//! 3. Move `BirthTranslation` from inside Shape to outside
//! 4. Convert field types as needed
//!
//! ## TODO
//! - [ ] Port from old applier.rs::apply_vfx_shape_fix (~160 LOC)
//! - [ ] Use filter::objects_by_type for initial matching
//! - [ ] This is the largest single transform — test carefully
