//! ChangeFieldType transform.
//!
//! Converts a field's value from one type to another (e.g. vec3 → vec4).
//! Uses ObjectFilter to find target objects and ValueFactory for conversion.
//!
//! ## Known conversion paths
//! - vec3 → vec4 (append values from config)
//! - link → string
//! - string → hash (compute FNV-1a)
//!
//! ## TODO
//! - [ ] Implement using filter::objects_by_type + factory::convert_type
//! - [ ] Port all 11 conversion paths from old applier.rs
