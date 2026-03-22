//! VfxShapeFix transform.
//!
//! Complex VFX shape structure migration for post-patch 14.1 changes.
//! This restructures VFX shape embeds by moving BirthTranslation out of Shape
//! and converting field formats.
//!
//! ## What it does
//! 1. Find `VfxSystemDefinitionData` objects
//! 2. Look for emitter containers (Complex/SimpleEmitterDefinitionData)
//! 3. Analyze Shape embeds for old-format fields
//! 4. Convert to new shape types (0x3dbe415d, 0xee39916f, or 0x4f4e2ed7)
//! 5. Move `BirthTranslation` from inside Shape to outside as sibling field
//!
//! ## New shape types
//! - `0x3dbe415d` — Cylinder with Radius/Height/Flags
//! - `0xee39916f` — Simple EmitOffset (Vec3)
//! - `0x4f4e2ed7` — Default fallback (empty)

use crate::context::FixContext;
use hematite_types::bin::{BinProperty, PropertyValue, StructValue};
use hematite_types::hash::{FieldHash, TypeHash};

const NEW_BIRTH_TRANSLATION_HASH: u32 = 0x563d4a22;
const BIRTH_TRANSLATION_TYPE_HASH: u32 = 0x68dc32b6;
const SHAPE_TYPE_CYLINDER: u32 = 0x3dbe415d;
const SHAPE_TYPE_SIMPLE: u32 = 0xee39916f;
const SHAPE_TYPE_DEFAULT: u32 = 0x4f4e2ed7;

struct ShapeAnalysis {
    needs_fix: bool,
    birth_translation_vec3: Option<[f32; 3]>,
    radius: f32,
    height: f32,
    has_cylinder_pattern: bool,
}

pub fn apply(ctx: &mut FixContext, entry_type: &str) -> u32 {
    let Some(vfx_system_hash) = ctx.hashes.type_hash(entry_type) else {
        return 0;
    };

    let Some(complex_emitter_hash) = ctx.hashes.type_hash("ComplexEmitterDefinitionData") else {
        return 0;
    };
    let Some(simple_emitter_hash) = ctx.hashes.type_hash("SimpleEmitterDefinitionData") else {
        return 0;
    };

    let hashes = VfxHashes {
        shape: ctx.hashes.field_hash("Shape"),
        birth_translation: ctx.hashes.field_hash("BirthTranslation"),
        emit_offset: ctx.hashes.field_hash("EmitOffset"),
        emit_rotation_angles: ctx.hashes.field_hash("EmitRotationAngles"),
        emit_rotation_axes: ctx.hashes.field_hash("EmitRotationAxes"),
        constant_value: ctx.hashes.field_hash("ConstantValue"),
        radius: ctx.hashes.field_hash("Radius"),
        height: ctx.hashes.field_hash("Height"),
        flags: ctx.hashes.field_hash("Flags"),
    };

    let Some(shape_hash) = hashes.shape else {
        return 0;
    };

    let mut changes = 0u32;
    let object_keys: Vec<u32> = ctx
        .tree
        .objects
        .keys()
        .filter(|&&path_hash| {
            ctx.tree
                .objects
                .get(&path_hash)
                .map(|obj| obj.class_hash == vfx_system_hash)
                .unwrap_or(false)
        })
        .copied()
        .collect();

    for path_hash in object_keys {
        let Some(obj) = ctx.tree.objects.get_mut(&path_hash) else {
            continue;
        };

        let prop_keys: Vec<u32> = obj.properties.keys().copied().collect();

        for prop_hash in prop_keys {
            let Some(prop) = obj.properties.get_mut(&prop_hash) else {
                continue;
            };

            if let PropertyValue::Container(emitters) = &mut prop.value {
                for emitter_val in emitters.iter_mut() {
                    if let PropertyValue::Embedded(emitter) = emitter_val {
                        let is_emitter = emitter.class_hash == complex_emitter_hash
                            || emitter.class_hash == simple_emitter_hash;

                        if !is_emitter {
                            continue;
                        }

                        if let Some(shape_prop) = emitter.properties.get_mut(&shape_hash.0) {
                            if let PropertyValue::Struct(shape) = &mut shape_prop.value {
                                let analysis = analyze_shape(shape, &hashes);

                                if analysis.needs_fix {
                                    apply_shape_conversion(shape, &analysis, &hashes);

                                    if let Some(birth_vec) = analysis.birth_translation_vec3 {
                                        move_birth_translation_outside(
                                            emitter,
                                            birth_vec,
                                            hashes.constant_value,
                                        );
                                    }

                                    changes += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    changes
}

struct VfxHashes {
    shape: Option<FieldHash>,
    birth_translation: Option<FieldHash>,
    emit_offset: Option<FieldHash>,
    emit_rotation_angles: Option<FieldHash>,
    emit_rotation_axes: Option<FieldHash>,
    constant_value: Option<FieldHash>,
    radius: Option<FieldHash>,
    height: Option<FieldHash>,
    flags: Option<FieldHash>,
}

fn analyze_shape(shape: &StructValue, hashes: &VfxHashes) -> ShapeAnalysis {
    let mut analysis = ShapeAnalysis {
        needs_fix: false,
        birth_translation_vec3: None,
        radius: 0.0,
        height: 0.0,
        has_cylinder_pattern: false,
    };

    for (field_hash, field_prop) in &shape.properties {
        if hashes
            .birth_translation
            .map(|h| *field_hash == h.0)
            .unwrap_or(false)
        {
            analysis.needs_fix = true;
            if let PropertyValue::Struct(bt_struct) = &field_prop.value {
                analysis.birth_translation_vec3 =
                    extract_constant_value_vec3(bt_struct, hashes.constant_value);
            }
        }

        if hashes
            .emit_offset
            .map(|h| *field_hash == h.0)
            .unwrap_or(false)
        {
            analysis.needs_fix = true;
            if let PropertyValue::Struct(eo_struct) = &field_prop.value {
                if let Some(vec3) = extract_constant_value_vec3(eo_struct, hashes.constant_value) {
                    analysis.radius = vec3[0];
                    analysis.height = vec3[1];
                }
            }
        }

        if hashes
            .emit_rotation_angles
            .map(|h| *field_hash == h.0)
            .unwrap_or(false)
        {
            analysis.needs_fix = true;
            analysis.has_cylinder_pattern = true;
        }

        if hashes
            .emit_rotation_axes
            .map(|h| *field_hash == h.0)
            .unwrap_or(false)
        {
            analysis.needs_fix = true;
            if let PropertyValue::Container(axes) = &field_prop.value {
                if axes.len() == 2 {
                    if let (PropertyValue::Vector3(v0), PropertyValue::Vector3(v1)) =
                        (&axes[0], &axes[1])
                    {
                        if v0[1] == 1.0
                            && v0[0] == 0.0
                            && v0[2] == 0.0
                            && v1[2] == 1.0
                            && v1[0] == 0.0
                            && v1[1] == 0.0
                        {
                            analysis.has_cylinder_pattern = true;
                        }
                    }
                }
            }
        }
    }

    analysis
}

fn extract_constant_value_vec3(
    struct_val: &StructValue,
    constant_value_hash: Option<FieldHash>,
) -> Option<[f32; 3]> {
    let cv_hash = constant_value_hash?;
    let prop = struct_val.properties.get(&cv_hash.0)?;
    if let PropertyValue::Vector3(vec) = &prop.value {
        Some(*vec)
    } else {
        None
    }
}

fn apply_shape_conversion(shape: &mut StructValue, analysis: &ShapeAnalysis, hashes: &VfxHashes) {
    let target_type = if analysis.has_cylinder_pattern && analysis.radius != 0.0 {
        SHAPE_TYPE_CYLINDER
    } else if shape.properties.len() == 1 && analysis.radius != 0.0 {
        SHAPE_TYPE_SIMPLE
    } else {
        SHAPE_TYPE_DEFAULT
    };

    shape.properties.clear();
    shape.class_hash = TypeHash(target_type);

    match target_type {
        SHAPE_TYPE_CYLINDER => {
            if analysis.radius != 0.0 {
                if let Some(r_hash) = hashes.radius {
                    shape.properties.insert(
                        r_hash.0,
                        BinProperty {
                            name_hash: r_hash,
                            value: PropertyValue::F32(analysis.radius),
                        },
                    );
                }
            }
            if analysis.height != 0.0 {
                if let Some(h_hash) = hashes.height {
                    shape.properties.insert(
                        h_hash.0,
                        BinProperty {
                            name_hash: h_hash,
                            value: PropertyValue::F32(analysis.height),
                        },
                    );
                }
            }
            if let Some(f_hash) = hashes.flags {
                shape.properties.insert(
                    f_hash.0,
                    BinProperty {
                        name_hash: f_hash,
                        value: PropertyValue::U8(1),
                    },
                );
            }
        }
        SHAPE_TYPE_SIMPLE => {
            if let Some(r_hash) = hashes.radius {
                shape.properties.insert(
                    r_hash.0,
                    BinProperty {
                        name_hash: r_hash,
                        value: PropertyValue::Vector3([analysis.radius, analysis.height, 0.0]),
                    },
                );
            }
        }
        SHAPE_TYPE_DEFAULT => {}
        _ => {}
    }
}

fn move_birth_translation_outside(
    emitter: &mut StructValue,
    birth_vec: [f32; 3],
    constant_value_hash: Option<FieldHash>,
) {
    let Some(cv_hash) = constant_value_hash else {
        return;
    };

    let mut birth_props = indexmap::IndexMap::new();
    birth_props.insert(
        cv_hash.0,
        BinProperty {
            name_hash: cv_hash,
            value: PropertyValue::Vector3(birth_vec),
        },
    );

    emitter.properties.insert(
        NEW_BIRTH_TRANSLATION_HASH,
        BinProperty {
            name_hash: FieldHash(NEW_BIRTH_TRANSLATION_HASH),
            value: PropertyValue::Struct(StructValue {
                class_hash: TypeHash(BIRTH_TRANSLATION_TYPE_HASH),
                properties: birth_props,
            }),
        },
    );
}
