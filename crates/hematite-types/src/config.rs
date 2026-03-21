//! Fix configuration schema — deserialized from `fix_config.json`.
//!
//! This module defines the JSON schema for fix rules. Each rule has:
//! - A **detection rule** that identifies when an issue exists
//! - A **transform action** that fixes the issue
//!
//! The schema is designed to be config-driven: new fixes can be added by
//! editing JSON without changing Rust code (for simple detection/transform patterns).
//!
//! ## Fix rules in current config
//! | Fix ID | Detection | Transform |
//! |--------|-----------|-----------|
//! | `healthbar_fix` | `MissingOrWrongField` | `EnsureField` |
//! | `staticmat_texturepath` | `FieldHashExists` | `RenameHash` |
//! | `staticmat_samplername` | `FieldHashExists` | `RenameHash` |
//! | `black_icons` | `StringExtensionNotInWad` | `ReplaceStringExtension` |
//! | `dds_to_tex` | `RecursiveStringExtensionNotInWad` | `ReplaceStringExtension` |
//! | `champion_bin_remover` | `EntryTypeExistsAny` | `RemoveFromWad` |
//! | `bnk_remover` | `BnkVersionNotIn` | `RemoveFromWad` |
//! | `vfx_shape_fix` | `VfxShapeNeedsFix` | `VfxShapeFix` |
//!
//! ## Future
//! - Port ContextualValues (champion/subchamp-specific overrides)
//! - Add unit tests for JSON round-tripping

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Root config structure loaded from fix_config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixConfig {
    pub version: String,
    pub last_updated: String,
    pub fixes: HashMap<String, FixRule>,
}

/// A single fix rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixRule {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub severity: String,
    pub detect: DetectionRule,
    pub apply: TransformAction,
}

/// How to detect an issue in a BIN file.
///
/// Uses serde internally-tagged enum: `"type": "missing_or_wrong_field"` etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DetectionRule {
    /// Field is missing or has the wrong value in a specific embed path.
    #[serde(rename = "missing_or_wrong_field")]
    MissingOrWrongField {
        entry_type: String,
        #[serde(default)]
        embed_path: Option<String>,
        #[serde(default)]
        embed_type: Option<String>,
        field: String,
        #[serde(default)]
        expected_value: Option<serde_json::Value>,
    },

    /// A field hash exists at a dot-separated path (e.g. "SamplerValues.*.TextureName").
    #[serde(rename = "field_hash_exists")]
    FieldHashExists {
        entry_type: String,
        path: String,
    },

    /// Strings with a given extension that don't exist in the WAD cache.
    #[serde(rename = "string_extension_not_in_wad")]
    StringExtensionNotInWad {
        entry_type: String,
        fields: Vec<String>,
        extension: String,
    },

    /// Recursive scan for strings with extension not in WAD (with path prefix filtering).
    #[serde(rename = "recursive_string_extension_not_in_wad")]
    RecursiveStringExtensionNotInWad {
        extension: String,
        #[serde(default)]
        path_prefixes: Vec<String>,
    },

    /// Any object in the BIN matches one of the given entry types.
    #[serde(rename = "entry_type_exists_any")]
    EntryTypeExistsAny {
        entry_types: Vec<String>,
    },

    /// BNK audio file version is not in the allowed list.
    #[serde(rename = "bnk_version_not_in")]
    BnkVersionNotIn {
        allowed_versions: Vec<u32>,
    },

    /// VFX shape data needs migration (post-patch 14.1 format change).
    #[serde(rename = "vfx_shape_needs_fix")]
    VfxShapeNeedsFix {
        entry_type: String,
    },
}

/// How to fix a detected issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransformAction {
    /// Add or update a field value (optionally creating parent embeds).
    #[serde(rename = "ensure_field")]
    EnsureField {
        field: String,
        value: serde_json::Value,
        data_type: String,
        #[serde(default)]
        create_parent: Option<ParentEmbed>,
    },

    /// Rename a field hash across the BIN tree.
    #[serde(rename = "rename_hash")]
    RenameHash {
        from_hash: String,
        to_hash: String,
    },

    /// Replace file extension in all string values (e.g. .dds → .tex).
    #[serde(rename = "replace_string_extension")]
    ReplaceStringExtension {
        from: String,
        to: String,
    },

    /// Mark file for removal from WAD.
    #[serde(rename = "remove_from_wad")]
    RemoveFromWad,

    /// Change a field's value type (e.g. vec3 → vec4, link → string).
    #[serde(rename = "change_field_type")]
    ChangeFieldType {
        from_type: String,
        to_type: String,
        #[serde(default)]
        conversion_rule: Option<String>,
        #[serde(default)]
        append_values: Vec<serde_json::Value>,
    },

    /// Regex-based string replacement.
    #[serde(rename = "regex_replace")]
    RegexReplace {
        pattern: String,
        replacement: String,
        #[serde(default)]
        field_filter: Option<String>,
    },

    /// Regex-based field rename with capture group support.
    #[serde(rename = "regex_rename_field")]
    RegexRenameField {
        pattern: String,
        replacement: String,
    },

    /// Complex VFX shape structure migration.
    #[serde(rename = "vfx_shape_fix")]
    VfxShapeFix,
}

/// Parent embed to create when EnsureField target doesn't exist yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentEmbed {
    pub field: String,
    #[serde(rename = "type")]
    pub embed_type: String,
}

/// All BIN data types for value creation.
///
/// ## TODO
/// - [ ] Map these to PropertyValue variants in ValueFactory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinDataType {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    Vector2,
    Vector3,
    Vector4,
    String,
    Hash,
    Link,
    Color,
}
