//! ChangeFieldType transform.
//!
//! Converts all PropertyValues of one type to another (e.g. vec3 → vec4).
//! Uses PropertyWalker with custom logic to check and convert values.
//!
//! ## Used by
//! - `vec3_to_vec4`: Adds alpha channel to 3D vectors
//! - `hash_to_string`: Converts hash fields to hex strings
//!
//! ## Type conversions (via factory::convert_type)
//! - vec2 → vec3/vec4 (with append_values)
//! - vec3 → vec4 (with alpha value)
//! - hash/link ↔ string (hex format)
//! - u8 ↔ u32 (clamped)
//! - f32 → string

use crate::context::FixContext;
use crate::factory::convert_type;
use hematite_types::bin::PropertyValue;

/// Apply ChangeFieldType transform to entire tree.
///
/// Walks all PropertyValues and attempts conversion using factory::convert_type.
/// Returns count of converted values.
pub fn apply(
    ctx: &mut FixContext,
    from_type: &str,
    to_type: &str,
    append_values: &[serde_json::Value],
) -> u32 {
    let mut changes = 0u32;

    for obj in ctx.tree.objects.values_mut() {
        for prop in obj.properties.values_mut() {
            changes += convert_value(&mut prop.value, from_type, to_type, append_values);
        }
    }

    changes
}

/// Recursively convert values matching the from_type.
fn convert_value(
    value: &mut PropertyValue,
    from_type: &str,
    to_type: &str,
    append_values: &[serde_json::Value],
) -> u32 {
    let mut changes = 0u32;

    match value {
        PropertyValue::Container(values) | PropertyValue::UnorderedContainer(values) => {
            for v in values {
                changes += convert_value(v, from_type, to_type, append_values);
            }
        }
        PropertyValue::Struct(struct_val) | PropertyValue::Embedded(struct_val) => {
            for prop in struct_val.properties.values_mut() {
                changes += convert_value(&mut prop.value, from_type, to_type, append_values);
            }
        }
        PropertyValue::Optional(inner) => {
            if let Some(v) = inner.as_mut() {
                changes += convert_value(v, from_type, to_type, append_values);
            }
        }
        PropertyValue::Map(entries) => {
            for (_k, v) in entries {
                changes += convert_value(v, from_type, to_type, append_values);
            }
        }
        _ => {
            if let Ok(Some(new_val)) = convert_type(value, from_type, to_type, append_values) {
                *value = new_val;
                changes += 1;
            }
        }
    }

    changes
}
