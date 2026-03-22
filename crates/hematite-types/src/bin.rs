//! BIN tree types — Hematite's own representation of League BIN files.
//!
//! These types mirror the structure of LTK's `BinTree` / `BinObject` / `PropertyValueEnum`
//! but are **owned by Hematite**. The LTK adapter crate converts between these and
//! whatever LTK version is currently in use, isolating the rest of the codebase from
//! LTK breaking changes.
//!
//! ## Key types
//! - [`BinTree`] — A parsed `.bin` file (map of path_hash → [`BinObject`])
//! - [`BinObject`] — A single object/entry in the tree
//! - [`BinProperty`] — A named property (field_hash + value)
//! - [`PropertyValue`] — The value of a property (enum over all League types)

use indexmap::IndexMap;
use crate::hash::{TypeHash, FieldHash, PathHash};

/// A parsed BIN file — a map of entry path hashes to objects.
#[derive(Debug, Clone, Default)]
pub struct BinTree {
    pub objects: IndexMap<u32, BinObject>,
}

/// A single object in a BIN tree.
#[derive(Debug, Clone)]
pub struct BinObject {
    /// Class hash identifying the object's type (e.g. SkinCharacterDataProperties).
    pub class_hash: TypeHash,
    /// Path hash of this entry.
    pub path_hash: PathHash,
    /// The object's properties, keyed by field name hash.
    pub properties: IndexMap<u32, BinProperty>,
}

/// A single property in a BIN object.
#[derive(Debug, Clone)]
pub struct BinProperty {
    /// Field name hash.
    pub name_hash: FieldHash,
    /// The property's value.
    pub value: PropertyValue,
}

/// All possible property value types in a BIN file.
///
/// This enum must be kept in sync with LTK's `PropertyValueEnum`.
/// The conversion happens in `hematite-ltk/src/convert.rs`.
///
/// All variants match LTK's PropertyValueEnum. The conversion between
/// LTK and Hematite types happens in `hematite-ltk/src/convert.rs`.
#[derive(Debug, Clone)]
pub enum PropertyValue {
    // Primitives
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    // Vectors
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Vector4([f32; 4]),
    // Matrices
    Matrix4x4([[f32; 4]; 4]),
    // Strings & hashes
    String(String),
    Hash(u32),
    WadHash(u64),
    // Links
    Link(u32),
    // Color
    Color([u8; 4]),
    // Nested structures
    Struct(StructValue),
    Embedded(StructValue),
    // Collections
    Container(Vec<PropertyValue>),
    UnorderedContainer(Vec<PropertyValue>),
    // Optional
    Optional(Box<Option<PropertyValue>>),
    // Map
    Map(Vec<(PropertyValue, PropertyValue)>),
    // Flags / bitfield
    BitBool(u8),
}

/// A struct-like value containing typed properties.
#[derive(Debug, Clone)]
pub struct StructValue {
    /// The class hash of this struct type.
    pub class_hash: TypeHash,
    /// Properties within the struct.
    pub properties: IndexMap<u32, BinProperty>,
}
