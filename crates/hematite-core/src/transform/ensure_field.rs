//! EnsureField + EnsureFieldWithContext transform.
//!
//! Adds or updates a field value in BIN objects. If the field's parent embed
//! doesn't exist, creates it first.
//!
//! ## Used by
//! - `healthbar_fix`: Adds `UnitHealthBarStyle = 12 (u8)` inside
//!   `HealthBarData: CharacterHealthBarDataRecord` embed
//!
//! ## Shared utils
//! - `filter::objects_by_type` — find objects of the target entry_type
//! - `factory::json_to_value` — convert JSON config value to PropertyValue
//!
//! ## TODO
//! - [ ] Implement field insertion with parent embed creation
//! - [ ] Implement EnsureFieldWithContext (champion-specific values)
//! - [ ] Port ParentEmbed creation logic from old applier.rs
