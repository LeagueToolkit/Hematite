//! EnsureField transform.
//!
//! Adds or updates a field value in BIN objects of a specific type.
//! If the field's parent embed doesn't exist, creates it first.
//!
//! ## Used by
//! - `healthbar_fix`: Adds `UnitHealthBarStyle = 12 (u8)` inside
//!   `HealthBarData: CharacterHealthBarDataRecord` embed
//!
//! ## Shared utils
//! - `filter::object_keys_by_type` — find objects of the target entry_type
//! - `factory::json_to_value` — convert JSON config value to PropertyValue

use crate::context::FixContext;
use crate::factory::json_to_value;
use crate::filter::object_keys_by_type;
use hematite_types::bin::{BinProperty, PropertyValue, StructValue};
use hematite_types::config::ParentEmbed;

/// Apply EnsureField transform to objects of the specified type.
///
/// Returns count of modified objects.
pub fn apply(
    ctx: &mut FixContext,
    entry_type: &str,
    field: &str,
    value: &serde_json::Value,
    data_type: &str,
    create_parent: Option<&ParentEmbed>,
) -> u32 {
    let Some(type_hash) = ctx.hashes.type_hash(entry_type) else {
        return 0;
    };

    let Some(field_hash) = ctx.hashes.field_hash(field) else {
        return 0;
    };

    let Ok(property_value) = json_to_value(value, data_type) else {
        return 0;
    };

    let object_keys = object_keys_by_type(&ctx.tree, type_hash);
    let mut changes = 0u32;

    for path_hash in object_keys {
        let Some(obj) = ctx.tree.objects.get_mut(&path_hash) else {
            continue;
        };

        if let Some(parent) = create_parent {
            changes += ensure_parent_embed(ctx.hashes, obj, parent, field_hash.0, &property_value);
        } else {
            obj.properties.insert(
                field_hash.0,
                BinProperty {
                    name_hash: field_hash,
                    value: property_value.clone(),
                },
            );
            changes += 1;
        }
    }

    changes
}

/// Ensure parent embed exists and set field inside it.
fn ensure_parent_embed(
    hashes: &dyn crate::traits::HashProvider,
    obj: &mut hematite_types::bin::BinObject,
    parent: &ParentEmbed,
    field_hash: u32,
    field_value: &PropertyValue,
) -> u32 {
    let Some(parent_hash) = hashes.field_hash(&parent.field) else {
        return 0;
    };

    let Some(embed_type_hash) = hashes.type_hash(&parent.embed_type) else {
        return 0;
    };

    let parent_prop = obj.properties.entry(parent_hash.0).or_insert_with(|| {
        BinProperty {
            name_hash: parent_hash,
            value: PropertyValue::Embedded(StructValue {
                class_hash: embed_type_hash,
                properties: Default::default(),
            }),
        }
    });

    // Accept both Embedded (correct for inline structs) and Struct (for pre-existing data)
    if let PropertyValue::Embedded(ref mut struct_val) | PropertyValue::Struct(ref mut struct_val) = parent_prop.value {
        struct_val.properties.insert(
            field_hash,
            BinProperty {
                name_hash: hematite_types::hash::FieldHash(field_hash),
                value: field_value.clone(),
            },
        );
        1
    } else {
        0
    }
}
