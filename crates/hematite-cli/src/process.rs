//! File processing orchestration.
//!
//! Routes input files to the appropriate processing pipeline based on file type.

use anyhow::{Context, Result};
use hematite_core::context::FixContext;
use hematite_core::pipeline::apply_fixes;
use hematite_core::traits::BinProvider;
use hematite_ltk::{bin_adapter::LtkBinProvider, hash_adapter::TxtHashProvider, wad_adapter::LtkWadProvider};
use hematite_types::config::FixConfig;
use hematite_types::result::ProcessResult;
use std::path::Path;
use walkdir::WalkDir;

/// Process input (file or directory).
pub fn process_input(
    input: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<ProcessResult> {
    let mut total_result = ProcessResult::default();

    if input.is_dir() {
        for entry in WalkDir::new(input) {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.is_file() && is_supported_file(path) {
                let result = process_file(path, config, selected_fixes, dry_run)?;
                total_result.merge(result);
            }
        }
    } else {
        total_result = process_file(input, config, selected_fixes, dry_run)?;
    }

    Ok(total_result)
}

/// Check if a file is a supported type.
fn is_supported_file(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_default();

    ext == "bin" || ext == "fantome" || ext == "zip" || file_name.ends_with(".wad.client")
}

/// Process a single file based on its type.
fn process_file(
    file: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<ProcessResult> {
    let ext = file.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let file_name = file.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_default();

    if ext == "bin" {
        process_bin_file(file, config, selected_fixes, dry_run)
    } else if file_name.ends_with(".wad.client") {
        process_wad_file(file, config, selected_fixes, dry_run)
    } else if ext == "fantome" || ext == "zip" {
        process_fantome_file(file, config, selected_fixes, dry_run)
    } else {
        anyhow::bail!("Unsupported file type: {}", file.display());
    }
}

/// Process a single .bin file.
fn process_bin_file(
    file: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<ProcessResult> {
    tracing::info!("Processing BIN: {}", file.display());

    // Initialize providers
    let hash_provider = TxtHashProvider::load_from_appdata()
        .context("Failed to load hash dictionaries")?;
    let bin_provider = LtkBinProvider;

    // Read BIN file
    let bytes = std::fs::read(file)
        .context("Failed to read BIN file")?;
    let tree = bin_provider.parse_bytes(&bytes)
        .context("Failed to parse BIN file")?;

    // Create dummy providers for standalone BIN (no WAD, no champions)
    struct NullWadProvider;
    impl hematite_core::traits::WadProvider for NullWadProvider {
        fn has_path(&self, _path: &str) -> bool { false }
        fn has_hash(&self, _hash: u64) -> bool { false }
    }
    let null_wad = NullWadProvider;

    // TODO: Load champion list from JSON
    let champions = hematite_types::champion::CharacterRelations::default();

    // Create fix context
    let mut ctx = FixContext {
        tree,
        hashes: &hash_provider,
        wad: &null_wad,
        champions: &champions,
        files_to_remove: Vec::new(),
        file_path: file.to_string_lossy().to_string(),
    };

    // Run fixes
    let result = apply_fixes(&mut ctx, config, selected_fixes, dry_run);

    // Write back if changes were made and not dry-run
    if !dry_run && result.fixes_applied > 0 {
        tracing::info!("Writing modified BIN file");

        // TODO: Implement write_bytes once LTK supports it
        // For now, just warn that writing is not yet supported
        tracing::warn!("⚠ BIN writing not yet implemented (LTK limitation)");
        tracing::info!("Dry-run mode would have written {} fixes", result.fixes_applied);
    }

    Ok(result)
}

/// Process a .wad.client file.
fn process_wad_file(
    file: &Path,
    _config: &FixConfig,
    _selected_fixes: &[String],
    _dry_run: bool,
) -> Result<ProcessResult> {
    tracing::info!("Processing WAD: {}", file.display());

    // Initialize providers
    let _hash_provider = TxtHashProvider::load_from_appdata()
        .context("Failed to load hash dictionaries")?;
    let wad_provider = LtkWadProvider::from_file(file)
        .context("Failed to load WAD file")?;

    // TODO: Extract BIN files from WAD, process each, rebuild WAD
    // For v2 MVP, this is deferred until LTK write support is ready

    tracing::warn!("⚠ WAD processing not yet implemented");
    tracing::info!("Detected WAD with {} entries", wad_provider.hash_count());

    Ok(ProcessResult::default())
}

/// Process a .fantome or .zip file.
fn process_fantome_file(
    file: &Path,
    _config: &FixConfig,
    _selected_fixes: &[String],
    _dry_run: bool,
) -> Result<ProcessResult> {
    tracing::info!("Processing Fantome: {}", file.display());

    // TODO: Extract ZIP, find .wad.client files, process, repack
    // For v2 MVP, this is deferred

    tracing::warn!("⚠ Fantome processing not yet implemented");

    Ok(ProcessResult::default())
}
