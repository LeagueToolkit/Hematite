//! Shader fallback transform — replaces invalid shader references with closest match.
//!
//! Uses token-based similarity matching from [`crate::detect::shader::ShaderValidator`]
//! to find the best replacement shader when a mod references a shader that no longer exists.

use crate::context::FixContext;
use crate::detect::shader::ShaderValidator;
use crate::filter;
use hematite_types::bin::PropertyValue;

/// Apply shader fallback: find invalid Link values in material definitions
/// and replace them with the closest valid shader hash.
///
/// Returns number of changes made.
pub fn apply(
    ctx: &mut FixContext<'_>,
    shader_def_type: &str,
    _shader_link_field: &str,
    shader_validator: &ShaderValidator,
) -> u32 {
    if !shader_validator.is_available() {
        tracing::debug!("Shader validator not available, skipping shader fallback");
        return 0;
    }

    let Some(target_type_hash) = ctx.hashes.type_hash(shader_def_type) else {
        return 0;
    };

    let object_keys: Vec<u32> = filter::objects_by_type(&ctx.tree, target_type_hash)
        .map(|obj| obj.path_hash.0)
        .collect();

    let mut changes = 0u32;

    for path_hash in object_keys {
        let Some(obj) = ctx.tree.objects.get_mut(&path_hash) else {
            continue;
        };

        // Walk all properties and fix invalid Link values
        let prop_keys: Vec<u32> = obj.properties.keys().cloned().collect();
        for prop_key in prop_keys {
            if let Some(prop) = obj.properties.get_mut(&prop_key) {
                changes += fix_shader_links(&mut prop.value, shader_validator);
            }
        }
    }

    if changes > 0 {
        tracing::info!(
            "Shader fallback: replaced {} invalid shader reference(s)",
            changes
        );
    }

    changes
}

/// Recursively walk a property value tree and fix invalid shader Link values.
fn fix_shader_links(value: &mut PropertyValue, validator: &ShaderValidator) -> u32 {
    let mut changes = 0;

    match value {
        PropertyValue::Link(hash) => {
            if *hash != 0 && !validator.is_valid_shader(*hash as u64) {
                // Try to find the shader path for this hash to do token matching
                let invalid_name = validator
                    .resolve_path(*hash as u64)
                    .unwrap_or("")
                    .to_string();

                if !invalid_name.is_empty() {
                    if let Some((new_path, new_hash)) = validator.find_closest_shader(&invalid_name)
                    {
                        tracing::info!(
                            "Shader fallback: {} → {} (hash: {:08X} → {:016X})",
                            invalid_name,
                            new_path,
                            hash,
                            new_hash
                        );
                        *hash = new_hash as u32;
                        changes += 1;
                    }
                }
            }
        }
        PropertyValue::Struct(s) | PropertyValue::Embedded(s) => {
            for prop in s.properties.values_mut() {
                changes += fix_shader_links(&mut prop.value, validator);
            }
        }
        PropertyValue::Container(items) | PropertyValue::UnorderedContainer(items) => {
            for item in items {
                changes += fix_shader_links(item, validator);
            }
        }
        PropertyValue::Optional(boxed) => {
            if let Some(inner) = boxed.as_mut() {
                changes += fix_shader_links(inner, validator);
            }
        }
        _ => {}
    }

    changes
}
