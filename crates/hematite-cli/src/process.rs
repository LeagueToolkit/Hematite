//! File processing orchestration.
//!
//! Routes input files to the appropriate processing pipeline based on file type.

use anyhow::{Context, Result};
use hematite_core::context::FixContext;
use hematite_core::pipeline::apply_fixes;
use hematite_core::traits::{BinProvider, HashProvider};
use hematite_ltk::{
    bin_adapter::LtkBinProvider,
    hash_adapter::TxtHashProvider,
    lmdb_hash_adapter::LmdbHashProvider,
};
use hematite_types::champion::CharacterRelations;
use hematite_types::config::FixConfig;
use hematite_types::result::ProcessResult;
use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;

/// Load hash provider with LMDB fallback to TXT.
fn load_hash_provider() -> Result<Arc<dyn HashProvider>> {
    // Auto-download LMDB if missing (skip version check if already exists)
    if let Err(e) = crate::hash_downloader::ensure_hashes_available(false) {
        tracing::warn!("Failed to auto-download hash database: {}", e);
        tracing::info!("Will attempt to use existing files");
    }

    // Try LMDB first
    match LmdbHashProvider::load_from_appdata() {
        Ok(provider) => {
            tracing::info!("Using LMDB hash provider");
            return Ok(Arc::new(provider));
        }
        Err(e) => {
            tracing::warn!("LMDB hash provider unavailable: {}", e);
            tracing::info!("Falling back to TXT hash provider");
        }
    }

    // Fallback to TXT
    let txt_provider = TxtHashProvider::load_from_appdata()
        .context("Failed to load hash dictionaries (both LMDB and TXT failed)")?;
    Ok(Arc::new(txt_provider))
}

/// Process input (file or directory).
pub fn process_input(
    input: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
) -> Result<ProcessResult> {
    // Load hash provider once for all files
    let hash_provider = load_hash_provider()?;

    let mut total_result = ProcessResult::default();

    if input.is_dir() {
        for entry in WalkDir::new(input) {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.is_file() && is_supported_file(path) {
                let result = process_file_with_hashes(
                    path,
                    config,
                    selected_fixes,
                    champions,
                    dry_run,
                    &hash_provider,
                )?;
                total_result.merge(result);
            }
        }
    } else {
        total_result = process_file_with_hashes(
            input,
            config,
            selected_fixes,
            champions,
            dry_run,
            &hash_provider,
        )?;
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

/// Process a single file based on its type (with hash provider).
fn process_file_with_hashes(
    file: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
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
        process_bin_file(file, config, selected_fixes, champions, dry_run, hash_provider)
    } else if file_name.ends_with(".wad.client") {
        process_wad_file(file, config, selected_fixes, champions, dry_run, hash_provider)
    } else if ext == "fantome" || ext == "zip" {
        process_fantome_file(file, config, selected_fixes, champions, dry_run, hash_provider)
    } else {
        anyhow::bail!("Unsupported file type: {}", file.display());
    }
}

/// Process a single .bin file.
fn process_bin_file(
    file: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    tracing::info!("Processing BIN: {}", file.display());

    // Initialize BIN provider
    let bin_provider = LtkBinProvider;

    // Read BIN file
    let bytes = std::fs::read(file)
        .context("Failed to read BIN file")?;
    let tree = bin_provider.parse_bytes(&bytes)
        .context("Failed to parse BIN file")?;

    // Standalone BIN has no WAD context
    struct NullWadProvider;
    impl hematite_core::traits::WadProvider for NullWadProvider {
        fn has_path(&self, _path: &str) -> bool { false }
        fn has_hash(&self, _hash: u64) -> bool { false }
    }
    let null_wad = NullWadProvider;

    // Create fix context
    let mut ctx = FixContext {
        tree,
        hashes: hash_provider.as_ref(),
        wad: &null_wad,
        champions,
        files_to_remove: Vec::new(),
        file_path: file.to_string_lossy().to_string(),
    };

    // Run fixes
    let result = apply_fixes(&mut ctx, config, selected_fixes, dry_run);

    // Write back if changes were made and not dry-run
    if !dry_run && result.fixes_applied > 0 {
        tracing::warn!("BIN writing not yet implemented (LTK limitation) - {} fixes detected but not persisted", result.fixes_applied);
    }

    Ok(result)
}

/// Process a .wad.client file.
///
/// Extracts files from the WAD, runs WAD-level and BIN-level fix pipelines,
/// and reports results. Writing modified files back is not yet supported.
fn process_wad_file(
    file: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    use hematite_ltk::wad_adapter::WadFile;
    use hematite_core::wad_pipeline;

    tracing::info!("Processing WAD: {}", file.display());

    let bin_provider = LtkBinProvider;

    let mut wad_file = WadFile::open(file)
        .context("Failed to open WAD file")?;

    let wad_provider = wad_file.build_provider();

    // Extract all files for WAD-level pipeline
    let all_files = wad_file.extract_all_files(hash_provider.as_ref())
        .context("Failed to extract files from WAD")?;

    let bin_chunks: Vec<_> = all_files.iter()
        .filter(|(path, _)| path.to_lowercase().ends_with(".bin"))
        .cloned()
        .collect();

    tracing::info!("WAD has {} total entries, {} BIN files", wad_provider.hash_count(), bin_chunks.len());

    let mut total_result = ProcessResult::default();
    let mut shared_files_to_remove = Vec::new();

    // === WAD-LEVEL PIPELINE ===
    // Run file-level fixes (BNK removal, format conversions, etc.)
    tracing::debug!("Running WAD-level pipeline...");
    let wad_output = wad_pipeline::apply_wad_fixes(&all_files, config, selected_fixes)?;

    // Collect files to remove from WAD-level fixes
    shared_files_to_remove.extend(wad_output.files_to_remove.clone());

    // Track WAD-level fixes applied
    for wad_fix in &wad_output.applied_fixes {
        tracing::info!("WAD-level fix '{}' affected {} files", wad_fix.fix_name, wad_fix.files_affected);
        total_result.fixes_applied += wad_fix.files_affected;
    }

    // Log file conversions (not yet implemented)
    if !wad_output.files_to_convert.is_empty() {
        tracing::warn!(
            "File format conversion not yet implemented - {} files would be converted",
            wad_output.files_to_convert.len()
        );
    }

    // === BIN-LEVEL PIPELINE ===

    // Process BIN files
    for (path, bytes) in &bin_chunks {
        let tree = match bin_provider.parse_bytes(bytes) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to parse BIN {path}: {e}");
                continue;
            }
        };

        let mut ctx = FixContext {
            tree,
            hashes: hash_provider.as_ref(),
            wad: &wad_provider,
            champions,
            files_to_remove: Vec::new(),
            file_path: path.clone(),
        };

        let result = apply_fixes(&mut ctx, config, selected_fixes, dry_run);
        total_result.merge(result);

        // Collect files marked for removal from this BIN context
        shared_files_to_remove.extend(ctx.files_to_remove);
    }

    // Update total files removed count
    total_result.files_removed = shared_files_to_remove.len() as u32;

    if !shared_files_to_remove.is_empty() {
        tracing::info!("Total files marked for removal: {}", shared_files_to_remove.len());
        if !dry_run {
            tracing::warn!("WAD writing not yet implemented - {} files would be removed but changes not persisted", shared_files_to_remove.len());
        }
    }

    if !dry_run && total_result.fixes_applied > 0 {
        tracing::warn!(
            "BIN writing not yet implemented (LTK limitation) - {} fixes detected in WAD but not persisted",
            total_result.fixes_applied
        );
    }

    Ok(total_result)
}

/// Process a .fantome or .zip file.
///
/// Extracts WAD files from the ZIP archive and processes each one.
fn process_fantome_file(
    file: &Path,
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    tracing::info!("Processing Fantome: {}", file.display());

    let zip_file = std::fs::File::open(file)
        .context("Failed to open fantome/zip file")?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(zip_file))
        .context("Failed to read ZIP archive")?;

    // Extract .wad.client files to temp dir
    let temp_dir = tempfile::tempdir()
        .context("Failed to create temp directory")?;

    let mut wad_paths = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .context("Failed to read ZIP entry")?;

        let name = entry.name().to_lowercase();
        if name.ends_with(".wad.client") {
            let dest = temp_dir.path().join(entry.name());
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
            wad_paths.push(dest);
        }
    }

    if wad_paths.is_empty() {
        tracing::warn!("No .wad.client files found in {}", file.display());
        return Ok(ProcessResult::default());
    }

    tracing::info!("Found {} WAD file(s) in archive", wad_paths.len());

    let mut total_result = ProcessResult::default();
    for wad_path in &wad_paths {
        let result = process_wad_file(wad_path, config, selected_fixes, champions, dry_run, hash_provider)?;
        total_result.merge(result);
    }

    Ok(total_result)
}
