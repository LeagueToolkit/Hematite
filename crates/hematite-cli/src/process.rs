//! File processing orchestration.
//!
//! Routes input files to the appropriate processing pipeline based on file type.

use anyhow::{Context, Result};
use hematite_core::context::FixContext;
use hematite_core::pipeline::apply_fixes;
use hematite_core::repath as repath_core;
use hematite_core::traits::{BinProvider, HashProvider};
use hematite_core::wad_pipeline::converters::ConverterRegistry;
use hematite_ltk::{
    bin_adapter::LtkBinProvider, hash_adapter::TxtHashProvider,
    lmdb_hash_adapter::LmdbHashProvider, mesh_converter, texture_converter,
    wad_adapter::wad_path_hash,
};
use hematite_types::champion::CharacterRelations;
use hematite_types::config::FixConfig;
use hematite_types::repath::RepathOptions;
use hematite_types::result::{CheckInfo, ProcessResult};
use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;

/// Session-level parameters shared by every file processing function.
///
/// Bundles together the options that are constant for the entire run so that
/// individual `process_*` functions stay within Clippy's argument-count limit.
struct ProcessContext<'a> {
    config: &'a FixConfig,
    selected_fixes: &'a [String],
    champions: &'a CharacterRelations,
    dry_run: bool,
    check: bool,
    repath_opts: Option<&'a RepathOptions>,
}

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
    check: bool,
    repath_opts: Option<&RepathOptions>,
) -> Result<ProcessResult> {
    // Load hash provider once for all files
    let hash_provider = load_hash_provider()?;

    let ctx = ProcessContext {
        config,
        selected_fixes,
        champions,
        dry_run,
        check,
        repath_opts,
    };

    let mut total_result = ProcessResult::default();

    if input.is_dir() {
        for entry in WalkDir::new(input) {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.is_file() && is_supported_file(path) {
                let result = process_file_with_hashes(path, &ctx, &hash_provider)?;
                total_result.merge(result);
            }
        }
    } else {
        total_result = process_file_with_hashes(input, &ctx, &hash_provider)?;
    }

    Ok(total_result)
}

/// Check if a file is a supported type.
fn is_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_default();

    ext == "bin" || ext == "fantome" || ext == "zip" || file_name.ends_with(".wad.client")
}

/// Process a single file based on its type (with hash provider).
fn process_file_with_hashes(
    file: &Path,
    ctx: &ProcessContext<'_>,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let file_name = file
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_default();

    if ext == "bin" {
        process_bin_file(file, ctx, hash_provider)
    } else if file_name.ends_with(".wad.client") {
        process_wad_file(file, ctx, hash_provider)
    } else if ext == "fantome" || ext == "zip" {
        process_fantome_file(file, ctx, hash_provider)
    } else {
        anyhow::bail!("Unsupported file type: {}", file.display());
    }
}

/// Process a single .bin file.
fn process_bin_file(
    file: &Path,
    ctx: &ProcessContext<'_>,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    let (config, selected_fixes, champions, dry_run, check) = (
        ctx.config,
        ctx.selected_fixes,
        ctx.champions,
        ctx.dry_run,
        ctx.check,
    );
    tracing::info!("Processing BIN: {}", file.display());

    // Initialize BIN provider
    let bin_provider = LtkBinProvider;

    // Read BIN file
    let bytes = std::fs::read(file).context("Failed to read BIN file")?;
    let tree = bin_provider
        .parse_bytes(&bytes)
        .context("Failed to parse BIN file")?;

    // Standalone BIN has no WAD context
    struct NullWadProvider;
    impl hematite_core::traits::WadProvider for NullWadProvider {
        fn has_path(&self, _path: &str) -> bool {
            false
        }
        fn has_hash(&self, _hash: u64) -> bool {
            false
        }
    }
    let null_wad = NullWadProvider;

    // Load shader validator (optional, graceful if unavailable)
    let shader_validator = hematite_core::detect::shader::ShaderValidator::load()
        .ok()
        .filter(|v| v.is_available());

    // Create fix context
    let mut ctx = FixContext {
        tree,
        hashes: hash_provider.as_ref(),
        wad: &null_wad,
        champions,
        files_to_remove: Vec::new(),
        file_path: file.to_string_lossy().to_string(),
        linked_trees: std::collections::HashMap::new(),
        shader_validator: shader_validator.as_ref(),
    };

    // Run fixes
    let mut result = apply_fixes(&mut ctx, config, selected_fixes, dry_run);

    // In check mode, populate CheckInfo from detected issues
    if check {
        let detected: Vec<String> = result
            .applied_fixes
            .iter()
            .map(|f| f.fix_name.clone())
            .collect();
        result.check_info = Some(CheckInfo {
            champion: None,
            skin_number: None,
            is_binless: true, // standalone BIN = no WAD context
            detected_issues: detected,
        });
    }

    // Write back if changes were made and not dry-run
    if !dry_run && result.fixes_applied > 0 {
        let modified_bytes = bin_provider
            .write_bytes(&ctx.tree)
            .context("Failed to write modified BIN file")?;

        // Write to output file (original.bin → original.fixed.bin)
        let output_path = file.with_extension("fixed.bin");
        std::fs::write(&output_path, &modified_bytes)
            .context("Failed to save modified BIN file")?;

        tracing::info!("✓ Wrote fixed BIN to: {}", output_path.display());

        // Log each applied fix
        for fix in &result.applied_fixes {
            tracing::info!(
                "  ✓ {} ({} changes)",
                fix.fix_name,
                fix.changes_count
            );
        }

        tracing::info!(
            "  Total: {} fixes, {} bytes written",
            result.fixes_applied,
            modified_bytes.len()
        );
    }

    Ok(result)
}

/// Process a .wad.client file.
///
/// Extracts files from the WAD, runs WAD-level and BIN-level fix pipelines,
/// and reports results. Writing modified files back is not yet supported.
fn process_wad_file(
    file: &Path,
    ctx: &ProcessContext<'_>,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    let (config, selected_fixes, champions, dry_run, check, repath_opts) = (
        ctx.config,
        ctx.selected_fixes,
        ctx.champions,
        ctx.dry_run,
        ctx.check,
        ctx.repath_opts,
    );
    use hematite_core::wad_pipeline;
    use hematite_ltk::wad_adapter::WadFile;

    tracing::info!("Processing WAD: {}", file.display());

    let bin_provider = LtkBinProvider;

    let mut wad_file = WadFile::open(file).context("Failed to open WAD file")?;

    let wad_provider = wad_file.build_provider();

    // Extract all files for WAD-level pipeline (mutable for conversions)
    let mut all_files = wad_file
        .extract_all_files(hash_provider.as_ref())
        .context("Failed to extract files from WAD")?;

    let bin_chunks: Vec<_> = all_files
        .iter()
        .filter(|(_hash, path, _bytes)| path.to_lowercase().ends_with(".bin"))
        .cloned()
        .collect();

    tracing::info!(
        "WAD has {} total entries, {} BIN files",
        wad_provider.hash_count(),
        bin_chunks.len()
    );

    let mut total_result = ProcessResult::default();
    let mut shared_files_to_remove = Vec::new();

    // === WAD-LEVEL PIPELINE ===
    // Run file-level fixes (BNK removal, format conversions, etc.)
    tracing::debug!("Running WAD-level pipeline...");
    let wad_output = wad_pipeline::apply_wad_fixes(&all_files, config, selected_fixes, hash_provider.as_ref())?;

    // Collect files to remove from WAD-level fixes
    shared_files_to_remove.extend(wad_output.files_to_remove.clone());

    // Track WAD-level fixes applied
    for wad_fix in &wad_output.applied_fixes {
        tracing::info!(
            "WAD-level fix '{}' affected {} files",
            wad_fix.fix_name,
            wad_fix.files_affected
        );
        total_result.fixes_applied += wad_fix.files_affected;
    }

    // Perform file format conversions
    let mut converter_registry = ConverterRegistry::new();
    // Register LTK-based converters (override placeholders)
    converter_registry.register("dds_to_tex", texture_converter::dds_to_tex);
    converter_registry.register("sco_to_scb", mesh_converter::sco_to_scb);

    let mut conversion_count = 0u32;
    if !wad_output.files_to_convert.is_empty() {
        tracing::info!(
            "Converting {} file formats...",
            wad_output.files_to_convert.len()
        );

        for conversion in &wad_output.files_to_convert {
            // Find the file in all_files
            if let Some((_, _, bytes)) = all_files.iter_mut().find(|(_, p, _)| p == &conversion.path) {
                match converter_registry.convert(&conversion.converter, bytes) {
                    Ok(converted_bytes) => {
                        let old_size = bytes.len();
                        *bytes = converted_bytes;
                        conversion_count += 1;
                        tracing::info!(
                            "✓ Converted {} from .{} to .{} ({} → {} bytes)",
                            conversion.path,
                            conversion.from_ext,
                            conversion.to_ext,
                            old_size,
                            bytes.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "✗ Converter '{}' failed for {}: {}",
                            conversion.converter,
                            conversion.path,
                            e
                        );
                    }
                }
            }
        }

        total_result.fixes_applied += conversion_count;
    }

    // === LINKED BIN RESOLUTION (BFS) ===
    // Parse all BINs, resolve linked dependencies from WAD files
    let mut parsed_bins: std::collections::HashMap<String, hematite_types::bin::BinTree> =
        std::collections::HashMap::new();
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();

    for (_hash, path, bytes) in &bin_chunks {
        match bin_provider.parse_bytes(bytes) {
            Ok(tree) => {
                for linked_path in &tree.linked {
                    if !parsed_bins.contains_key(linked_path) {
                        queue.push_back(linked_path.clone());
                    }
                }
                parsed_bins.insert(path.clone(), tree);
            }
            Err(e) => {
                tracing::warn!("Failed to parse BIN {path}: {e}");
            }
        }
    }

    // BFS: resolve linked dependencies that exist in the WAD
    while let Some(linked_path) = queue.pop_front() {
        if parsed_bins.contains_key(&linked_path) {
            continue;
        }
        // Try to find this linked BIN in the extracted files
        if let Some((_, _, bytes)) = all_files.iter().find(|(_, p, _)| *p == linked_path) {
            match bin_provider.parse_bytes(bytes) {
                Ok(tree) => {
                    for dep in &tree.linked {
                        if !parsed_bins.contains_key(dep) {
                            queue.push_back(dep.clone());
                        }
                    }
                    tracing::debug!("Resolved linked BIN: {}", linked_path);
                    parsed_bins.insert(linked_path, tree);
                }
                Err(e) => {
                    tracing::debug!("Failed to parse linked BIN {}: {}", linked_path, e);
                }
            }
        } else {
            tracing::debug!("Linked BIN not found in WAD: {}", linked_path);
        }
    }

    // Separate primary BINs (from bin_chunks) from linked-only trees
    let primary_bin_paths: std::collections::HashSet<String> =
        bin_chunks.iter().map(|(_, p, _)| p.clone()).collect();
    let linked_only: std::collections::HashMap<String, hematite_types::bin::BinTree> = parsed_bins
        .iter()
        .filter(|(k, _)| !primary_bin_paths.contains(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // === BIN-LEVEL PIPELINE ===

    // Load shader validator once for all BIN files
    let shader_validator = hematite_core::detect::shader::ShaderValidator::load()
        .ok()
        .filter(|v| v.is_available());

    // Process primary BIN files
    for (_, path, _) in &bin_chunks {
        let Some(tree) = parsed_bins.remove(path) else {
            continue; // Already warned during parse
        };

        let mut ctx = FixContext {
            tree,
            hashes: hash_provider.as_ref(),
            wad: &wad_provider,
            champions,
            files_to_remove: Vec::new(),
            file_path: path.clone(),
            linked_trees: linked_only.clone(),
            shader_validator: shader_validator.as_ref(),
        };

        let result = apply_fixes(&mut ctx, config, selected_fixes, dry_run);

        // Log fixes applied to this specific BIN
        if result.fixes_applied > 0 {
            tracing::info!("  {} - {} fixes applied:", path, result.fixes_applied);
            for fix in &result.applied_fixes {
                tracing::info!(
                    "    ✓ {} ({} changes)",
                    fix.fix_name,
                    fix.changes_count
                );
            }
        }

        let fixes_applied = result.fixes_applied;
        total_result.merge(result);

        // Write modified BIN back to all_files collection
        if !dry_run && fixes_applied > 0 {
            match bin_provider.write_bytes(&ctx.tree) {
                Ok(modified_bytes) => {
                    // Update the BIN bytes in all_files
                    if let Some((_, _, file_bytes)) = all_files.iter_mut().find(|(_, p, _)| p == path) {
                        let old_size = file_bytes.len();
                        *file_bytes = modified_bytes;
                        tracing::debug!(
                            "Updated BIN {} in WAD ({} → {} bytes)",
                            path,
                            old_size,
                            file_bytes.len()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to write modified BIN {}: {}", path, e);
                }
            }
        }

        // Collect files marked for removal from this BIN context
        shared_files_to_remove.extend(ctx.files_to_remove);
    }

    // Update total files removed count
    total_result.files_removed = shared_files_to_remove.len() as u32;

    // === REPATH PIPELINE ===
    // Must run AFTER all BIN fixes so fixes operate on original paths.
    if let Some(opts) = repath_opts {
        if !dry_run {
            let prefix = &opts.prefix;
            tracing::info!("Repathing assets with prefix \"{}\"...", prefix);

            // 1. Repath all string references inside BIN files.
            let mut all_new_paths: Vec<String> = Vec::new();
            let mut repath_bin_count = 0u32;

            for (_, path, bytes) in all_files.iter_mut() {
                if !path.to_lowercase().ends_with(".bin") {
                    continue;
                }
                match bin_provider.parse_bytes(bytes) {
                    Ok(mut tree) => {
                        let result = repath_core::repath_bin_strings(&mut tree, prefix, opts.skip_vo);
                        if result.strings_repathed > 0 {
                            match bin_provider.write_bytes(&tree) {
                                Ok(new_bytes) => {
                                    repath_bin_count += result.strings_repathed;
                                    all_new_paths.extend(result.new_paths);
                                    *bytes = new_bytes;
                                    tracing::debug!(
                                        "Repathed {} strings in {}",
                                        result.strings_repathed,
                                        path
                                    );
                                }
                                Err(e) => tracing::warn!("Failed to write repathed BIN {}: {}", path, e),
                            }
                        }
                    }
                    Err(e) => tracing::warn!("Failed to parse BIN for repathing {}: {}", path, e),
                }
            }

            // 2. Rename non-BIN WAD files to their repathed paths.
            let mut repath_wad_count = 0u32;
            let repathed: Vec<(u64, String, Vec<u8>)> = all_files
                .drain(..)
                .map(|(hash, path, bytes)| {
                    if let Some(new_path) = repath_core::repath_wad_path(&path, prefix) {
                        let new_hash = wad_path_hash(&new_path);
                        repath_wad_count += 1;
                        (new_hash, new_path, bytes)
                    } else {
                        (hash, path, bytes)
                    }
                })
                .collect();
            all_files = repathed;

            tracing::info!(
                "  Repathed {} BIN string(s), {} WAD file(s)",
                repath_bin_count,
                repath_wad_count
            );

            if repath_bin_count > 0 || repath_wad_count > 0 {
                total_result.fixes_applied += 1;
            }

            // 3. Inject invisible placeholder textures for missing references.
            if opts.invis_texture && !all_new_paths.is_empty() {
                let existing_paths: std::collections::HashSet<String> =
                    all_files.iter().map(|(_, p, _)| p.to_lowercase()).collect();

                let placeholders =
                    repath_core::missing_invis_placeholders(&existing_paths, &all_new_paths);

                if !placeholders.is_empty() {
                    tracing::info!("  Injecting {} invis placeholder(s)...", placeholders.len());
                    for (path, bytes) in placeholders {
                        let hash = wad_path_hash(&path);
                        tracing::debug!("  + invis placeholder: {}", path);
                        all_files.push((hash, path, bytes));
                    }
                }
            }
        } else {
            tracing::info!(
                "[dry-run] Would repath assets with prefix \"{}\"{}",
                opts.prefix,
                if opts.invis_texture { " + invis placeholders" } else { "" }
            );
        }
    }

    // In check mode, populate CheckInfo with skin detection
    if check {
        use hematite_core::detect::skin::SkinDetector;

        let all_paths: Vec<String> = all_files.iter().map(|(_, p, _)| p.clone()).collect();
        let detector = SkinDetector::new();
        let skin_info = detector.detect_from_paths(&all_paths);

        let detected: Vec<String> = total_result
            .applied_fixes
            .iter()
            .map(|f| f.fix_name.clone())
            .collect();

        let skin_number = skin_info.primary_skin();
        let is_binless = skin_info.is_binless;
        let champion = if skin_info.champion.is_empty() {
            None
        } else {
            Some(skin_info.champion)
        };

        total_result.check_info = Some(CheckInfo {
            champion,
            skin_number,
            is_binless,
            detected_issues: detected,
        });
    }

    // === WAD REBUILDING ===
    // Write modified WAD if any changes were made and not dry-run
    if !dry_run && (total_result.fixes_applied > 0 || !shared_files_to_remove.is_empty()) {
        tracing::info!("Building modified WAD...");

        let output_path = file.with_extension("fixed.wad.client");
        let mut output_file =
            std::fs::File::create(&output_path).context("Failed to create output WAD file")?;

        let chunks_included =
            hematite_ltk::wad_builder::build_wad(&all_files, &shared_files_to_remove, &mut output_file)
                .context("Failed to build output WAD")?;

        tracing::info!("✓ Wrote fixed WAD to: {}", output_path.display());
        tracing::info!(
            "  {} chunks included, {} files removed",
            chunks_included,
            shared_files_to_remove.len()
        );
        tracing::info!("  {} total fixes applied", total_result.fixes_applied);
    } else if !dry_run {
        tracing::info!("No changes detected - WAD not modified");
    }

    Ok(total_result)
}

/// Process a .fantome or .zip file.
///
/// Extracts WAD files from the ZIP archive and processes each one.
fn process_fantome_file(
    file: &Path,
    ctx: &ProcessContext<'_>,
    hash_provider: &Arc<dyn HashProvider>,
) -> Result<ProcessResult> {
    let dry_run = ctx.dry_run;
    tracing::info!("Processing Fantome: {}", file.display());

    let zip_file = std::fs::File::open(file).context("Failed to open fantome/zip file")?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(zip_file))
        .context("Failed to read ZIP archive")?;

    // Extract .wad.client files to temp dir
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

    // SECURITY: Limits to prevent DoS attacks (ZIP bombs, memory exhaustion)
    const MAX_ENTRIES: usize = 1000;
    const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024; // 500MB per file
    const MAX_TOTAL_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB total

    if archive.len() > MAX_ENTRIES {
        anyhow::bail!(
            "ZIP archive contains too many entries ({} > {}). Possible ZIP bomb attack.",
            archive.len(),
            MAX_ENTRIES
        );
    }

    let mut total_extracted_size: u64 = 0;
    let mut wad_paths = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).context("Failed to read ZIP entry")?;

        let name = entry.name().to_lowercase();
        if name.ends_with(".wad.client") {
            // SECURITY: Validate ZIP entry path to prevent path traversal attacks
            let entry_name = entry.name();

            // Check for path traversal patterns
            if entry_name.contains("..") || std::path::Path::new(entry_name).is_absolute() {
                anyhow::bail!(
                    "Invalid ZIP entry path (potential path traversal): {}",
                    entry_name
                );
            }

            // Additional check: ensure no path component is exactly ".."
            if entry_name.split('/').any(|component| component == "..")
                || entry_name.split('\\').any(|component| component == "..")
            {
                anyhow::bail!(
                    "Invalid ZIP entry path (contains .. component): {}",
                    entry_name
                );
            }

            // SECURITY: Check uncompressed size before extraction
            let uncompressed_size = entry.size();
            if uncompressed_size > MAX_FILE_SIZE {
                anyhow::bail!(
                    "ZIP entry '{}' is too large ({} bytes > {} bytes limit). Possible ZIP bomb.",
                    entry_name,
                    uncompressed_size,
                    MAX_FILE_SIZE
                );
            }

            // SECURITY: Check total extracted size
            total_extracted_size = total_extracted_size.saturating_add(uncompressed_size);
            if total_extracted_size > MAX_TOTAL_SIZE {
                anyhow::bail!(
                    "Total extracted size exceeds limit ({} bytes > {} bytes). Possible ZIP bomb.",
                    total_extracted_size,
                    MAX_TOTAL_SIZE
                );
            }

            let dest = temp_dir.path().join(entry_name);

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
        let result = process_wad_file(wad_path, ctx, hash_provider)?;
        total_result.merge(result);
    }

    // === FANTOME REPACK ===
    // Rebuild the fantome ZIP with fixed WADs replacing the originals
    if !dry_run && total_result.fixes_applied > 0 {
        let output_path = file.with_extension("fixed.fantome");

        tracing::info!("Repacking fantome archive...");

        // Re-open the original ZIP to copy non-WAD entries
        let original_zip_file =
            std::fs::File::open(file).context("Failed to re-open original fantome")?;
        let mut original_archive =
            zip::ZipArchive::new(std::io::BufReader::new(original_zip_file))
                .context("Failed to re-read original ZIP")?;

        let output_file =
            std::fs::File::create(&output_path).context("Failed to create output fantome")?;
        let mut zip_writer = zip::ZipWriter::new(std::io::BufWriter::new(output_file));

        let zip_options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for i in 0..original_archive.len() {
            let mut entry = original_archive.by_index(i)?;
            let entry_name = entry.name().to_string();
            let is_wad = entry_name.to_lowercase().ends_with(".wad.client");

            if is_wad {
                // Use the fixed WAD if it exists, otherwise copy original
                let fixed_wad_path = temp_dir
                    .path()
                    .join(&entry_name)
                    .with_extension("fixed.wad.client");

                if fixed_wad_path.exists() {
                    let fixed_bytes = std::fs::read(&fixed_wad_path)
                        .context("Failed to read fixed WAD from temp")?;
                    zip_writer.start_file(&entry_name, zip_options)?;
                    std::io::Write::write_all(&mut zip_writer, &fixed_bytes)?;
                    tracing::debug!("Repacked fixed WAD: {}", entry_name);
                } else {
                    // No fixes applied to this WAD, copy original
                    zip_writer.start_file(&entry_name, zip_options)?;
                    std::io::copy(&mut entry, &mut zip_writer)?;
                    tracing::debug!("Repacked original WAD: {}", entry_name);
                }
            } else {
                // Copy non-WAD entries as-is (META/info.json, etc.)
                zip_writer.start_file(&entry_name, zip_options)?;
                std::io::copy(&mut entry, &mut zip_writer)?;
            }
        }

        zip_writer.finish()?;

        tracing::info!("✓ Wrote fixed fantome to: {}", output_path.display());
        tracing::info!("  {} total fixes applied", total_result.fixes_applied);
    } else if !dry_run {
        tracing::info!("No changes detected - fantome not modified");
    }

    Ok(total_result)
}
