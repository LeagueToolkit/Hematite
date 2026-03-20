//! ValueFactory — JSON → PropertyValue conversion.
//!
//! Centralizes value creation that was previously scattered across `applier.rs`.
//! The fix config specifies values as JSON; this module converts them to
//! [`PropertyValue`] instances based on the declared [`BinDataType`].
//!
//! Also handles type conversions (e.g. vec3 → vec4) for the `ChangeFieldType`
//! transform action.
//!
//! ## TODO
//! - [ ] Implement json_to_value() for all BinDataType variants
//! - [ ] Implement convert_type() for known conversion paths:
//!   vec3→vec4, link→string, string→hash, etc.
//! - [ ] Implement matches_json() for detection value comparison

use anyhow::Result;
use hematite_types::bin::PropertyValue;

/// Convert a JSON value to a PropertyValue based on the declared data type.
///
/// ## TODO
/// - [ ] Handle all types: bool, u8, i8, u16, i16, u32, i32, u64, i64, f32,
///   vector2, vector3, vector4, string, hash, link, color
pub fn json_to_value(
    _value: &serde_json::Value,
    _data_type: &str,
) -> Result<PropertyValue> {
    // TODO: Match on data_type string, parse JSON value accordingly
    anyhow::bail!("json_to_value not yet implemented")
}

/// Convert a PropertyValue from one type to another.
///
/// Known conversions:
/// - vec3 → vec4 (append values from config)
/// - link → string (hash to string representation)
/// - string → hash (compute FNV-1a)
///
/// ## TODO
/// - [ ] Implement all conversion paths from old applier.rs::convert_property_type
pub fn convert_type(
    _value: &PropertyValue,
    _from: &str,
    _to: &str,
    _append_values: &[serde_json::Value],
) -> Result<Option<PropertyValue>> {
    // TODO: Match on (from, to) pair, perform conversion
    anyhow::bail!("convert_type not yet implemented")
}

/// Check if a PropertyValue matches a JSON expected value.
///
/// Used by detection rules to compare current field values against expected.
///
/// ## TODO
/// - [ ] Handle numeric comparisons (u8, i32, f32, etc.)
/// - [ ] Handle string comparison
/// - [ ] Handle nested value comparison
pub fn matches_json(_value: &PropertyValue, _expected: &serde_json::Value) -> bool {
    // TODO: Implement value comparison
    false
}
