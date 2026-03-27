//! Fix configuration schema — deserialized from `fix_config.json`.
//!
//! This module defines the JSON schema for fix rules. Each rule has:
//! - A **detection rule** that identifies when an issue exists
//! - A **transform action** that fixes the issue
//!
//! The schema is designed to be config-driven: new fixes can be added by
//! editing JSON without changing Rust code (for simple detection/transform patterns).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root config structure loaded from fix_config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixConfig {
    pub version: String,
    pub last_updated: String,
    /// BIN-level fixes (operate on parsed BIN trees)
    pub fixes: HashMap<String, FixRule>,
    /// WAD-level fixes (operate on files before BIN parsing)
    #[serde(default)]
    pub wad_fixes: HashMap<String, WadFixRule>,
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
    FieldHashExists { entry_type: String, path: String },

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
    EntryTypeExistsAny { entry_types: Vec<String> },

    /// BNK audio file version is not in the allowed list.
    #[serde(rename = "bnk_version_not_in")]
    BnkVersionNotIn { allowed_versions: Vec<u32> },

    /// VFX shape data needs migration (post-patch 14.1 format change).
    #[serde(rename = "vfx_shape_needs_fix")]
    VfxShapeNeedsFix { entry_type: String },

    /// Shader references that don't exist in the valid shader list.
    #[serde(rename = "invalid_shader_reference")]
    InvalidShaderReference {
        shader_def_type: String,
        shader_link_field: String,
    },

    /// Entries of specific types not referenced by the main skin entry.
    #[serde(rename = "unreferenced_entry_of_type")]
    UnreferencedEntryOfType {
        main_entry_type: String,
        targets: Vec<EntryValidationTarget>,
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
    RenameHash { from_hash: String, to_hash: String },

    /// Replace file extension in all string values (e.g. .dds → .tex).
    #[serde(rename = "replace_string_extension")]
    ReplaceStringExtension {
        from: String,
        to: String,
        #[serde(default)]
        path_prefixes: Vec<String>,
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

    /// Replace invalid shader references with closest valid match.
    #[serde(rename = "shader_fallback")]
    ShaderFallback {
        shader_def_type: String,
        shader_link_field: String,
    },

    /// Remove entries not referenced by the main skin entry.
    #[serde(rename = "remove_unreferenced_entries")]
    RemoveUnreferencedEntries {
        main_entry_type: String,
        targets: Vec<EntryValidationTarget>,
    },
}

/// Parent embed to create when EnsureField target doesn't exist yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentEmbed {
    pub field: String,
    #[serde(rename = "type")]
    pub embed_type: String,
}

/// Target entry type for entry validation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryValidationTarget {
    /// The entry type to validate (e.g. "ContextualActionData").
    pub entry_type: String,
    /// Optional hex type hash for direct matching (e.g. "0xCF3A2F44").
    #[serde(default)]
    pub type_hash: Option<String>,
    /// Field name in the main entry that references this type.
    pub reference_field: String,
    /// Hash of the link field (hex string like "0xd8f64a0d").
    pub link_field: String,
}

// ============================================================================
// WAD-LEVEL FIXES (File operations before BIN parsing)
// ============================================================================

/// A WAD-level fix rule for file operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadFixRule {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub severity: String,
    pub detect: WadDetectionRule,
    pub apply: WadTransformAction,
}

/// How to detect issues at the WAD file level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WadDetectionRule {
    /// Match files by extension and optionally check binary headers.
    #[serde(rename = "file_extension")]
    FileExtension {
        extension: String,
        #[serde(default)]
        binary_check: Option<BinaryHeaderCheck>,
        /// List of filenames to exclude (e.g., ["sfx_events.bnk"])
        #[serde(default)]
        exclude_files: Vec<String>,
    },

    /// Match files by path pattern (glob-style).
    #[serde(rename = "file_pattern")]
    FilePattern {
        pattern: String,
        #[serde(default)]
        binary_check: Option<BinaryHeaderCheck>,
    },
}

/// Binary header validation for file format checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BinaryHeaderCheck {
    /// Check version number at specific offset.
    #[serde(rename = "version_at_offset")]
    VersionAtOffset {
        /// Byte offset in file
        offset: usize,
        /// Size in bytes (1, 2, or 4)
        size: usize,
        /// Byte order
        #[serde(default = "default_endian")]
        endian: Endian,
        /// List of allowed versions
        allowed_versions: Vec<u32>,
    },

    /// Check magic signature at start of file.
    #[serde(rename = "magic_signature")]
    MagicSignature {
        /// Expected bytes at start of file
        signature: Vec<u8>,
    },
}

fn default_endian() -> Endian {
    Endian::Little
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Endian {
    Little,
    Big,
}

/// How to transform files at the WAD level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WadTransformAction {
    /// Remove the file from WAD.
    #[serde(rename = "remove_file")]
    RemoveFile,

    /// Convert file format (e.g. DDS→TEX, SCO→SCB).
    #[serde(rename = "convert_format")]
    ConvertFormat {
        /// Source extension
        from_ext: String,
        /// Target extension
        to_ext: String,
        /// Converter name (must be registered in converter registry)
        converter: String,
    },

    /// Rename file (change path/extension).
    #[serde(rename = "rename_file")]
    RenameFile {
        /// Regex pattern to match
        pattern: String,
        /// Replacement string (supports $1, $2 capture groups)
        replacement: String,
    },
}

/// All BIN data types for value creation.
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
