//! WAD-level fix pipeline — file operations before BIN parsing.
//!
//! This pipeline handles file-level operations that don't require parsing BIN files:
//! - File removal based on version/format checks
//! - File format conversions (DDS→TEX, SCO→SCB)
//! - File renaming/path transformations
//!
//! ## Architecture
//! ```text
//! WAD file → extract file list
//!          → for each file:
//!             - check WadDetectionRule
//!             - if match: apply WadTransformAction
//!          → return modified file list
//! ```
//!
//! ## Modules
//! - [`detect`] — WAD-level detection (extension, binary headers)
//! - [`transform`] — WAD-level actions (remove, convert, rename)
//! - [`converters`] — File format converters registry

pub mod detect;
pub mod transform;
pub mod converters;

use anyhow::Result;
use hematite_types::config::{FixConfig, WadFixRule};

/// Result of applying a single WAD-level fix.
#[derive(Debug, Clone)]
pub struct WadFixResult {
    pub fix_id: String,
    pub fix_name: String,
    pub files_affected: u32,
}

/// Apply WAD-level fixes to a list of files.
///
/// Returns a list of file operations to perform (remove, convert, rename).
pub fn apply_wad_fixes(
    files: &[(String, Vec<u8>)],
    config: &FixConfig,
    selected_fix_ids: &[String],
) -> Result<WadFixOutput> {
    let mut output = WadFixOutput::default();

    for fix_id in selected_fix_ids {
        let Some(fix_rule) = config.wad_fixes.get(fix_id) else {
            continue;
        };

        if !fix_rule.enabled {
            continue;
        }

        let result = apply_single_fix(files, fix_rule, fix_id)?;
        output.merge(result);
    }

    Ok(output)
}

/// Output of WAD-level fix pipeline.
#[derive(Debug, Default, Clone)]
pub struct WadFixOutput {
    /// Files to remove (by path)
    pub files_to_remove: Vec<String>,
    /// Files to convert (path, from_ext, to_ext, converter_name)
    pub files_to_convert: Vec<FileConversion>,
    /// Files to rename (old_path, new_path)
    pub files_to_rename: Vec<(String, String)>,
    /// Applied fixes summary
    pub applied_fixes: Vec<WadFixResult>,
}

#[derive(Debug, Clone)]
pub struct FileConversion {
    pub path: String,
    pub from_ext: String,
    pub to_ext: String,
    pub converter: String,
}

impl WadFixOutput {
    fn merge(&mut self, other: WadFixOutput) {
        self.files_to_remove.extend(other.files_to_remove);
        self.files_to_convert.extend(other.files_to_convert);
        self.files_to_rename.extend(other.files_to_rename);
        self.applied_fixes.extend(other.applied_fixes);
    }
}

fn apply_single_fix(
    files: &[(String, Vec<u8>)],
    fix_rule: &WadFixRule,
    fix_id: &str,
) -> Result<WadFixOutput> {
    let mut output = WadFixOutput::default();
    let mut files_affected = 0u32;

    for (path, bytes) in files {
        // Check if this file matches the detection rule
        if detect::check_file(path, bytes, &fix_rule.detect)? {
            // Apply the transform action
            let action_result = transform::apply_action(path, bytes, &fix_rule.apply)?;

            match action_result {
                transform::ActionResult::RemoveFile => {
                    output.files_to_remove.push(path.clone());
                    files_affected += 1;
                }
                transform::ActionResult::ConvertFile { from_ext, to_ext, converter } => {
                    output.files_to_convert.push(FileConversion {
                        path: path.clone(),
                        from_ext,
                        to_ext,
                        converter,
                    });
                    files_affected += 1;
                }
                transform::ActionResult::RenameFile { new_path } => {
                    output.files_to_rename.push((path.clone(), new_path));
                    files_affected += 1;
                }
                transform::ActionResult::NoOp => {}
            }
        }
    }

    if files_affected > 0 {
        output.applied_fixes.push(WadFixResult {
            fix_id: fix_id.to_string(),
            fix_name: fix_rule.name.clone(),
            files_affected,
        });
    }

    Ok(output)
}
