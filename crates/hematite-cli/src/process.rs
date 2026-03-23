//! File processing orchestration.
//!
//! Routes input files to the appropriate processing pipeline based on file type.

use anyhow::{Context, Result};
use hematite_core::context::FixContext;
use hematite_core::pipeline::apply_fixes;
use hematite_core::traits::{BinProvider, HashProvider};
use hematite_core::wad_pipeline::converters::ConverterRegistry;
use hematite_ltk::{
    bin_adapter::LtkBinProvider, hash_adapter::TxtHashProvider,
    lmdb_hash_adapter::LmdbHashProvider, mesh_converter, texture_converter,
};
use hematite_types::champion::CharacterRelations;
use hematite_types::config::FixConfig;
use hematite_types::result::{CheckInfo, ProcessResult};
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
    check: bool,
) -> Result<ProcessResult> {
    // Load hash provider once for all files
    let hash_provider = load_hash_provider()?;

    let mut total_result = ProcessResult::default();

    if input.is_dir() {
        // Collect all files first to show progress
        let files: Vec<_> = WalkDir::new(input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && is_supported_file(e.path()))
            .map(|e| e.path().to_path_buf())
            .collect();

        if files.is_empty() {
            tracing::warn!("No supported files found in directory");
            return Ok(total_result);
        }

        // Log batch processing start
        crate::logging::log_batch_start(files.len());

        // Process each file with progress
        for (index, path) in files.iter().enumerate() {
            crate::logging::log_file_progress(index + 1, files.len(), &path.display().to_string());

            match process_file_with_hashes(
                path,
                config,
                selected_fixes,
                champions,
                dry_run,
                &hash_provider,
                check,
            ) {
                Ok(result) => {
                    let fixes_applied = result.fixes_applied;
                    let success = result.errors.is_empty();
                    total_result.merge(result);
                    crate::logging::log_file_complete(
                        &path.display().to_string(),
                        fixes_applied,
                        success,
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to process {}: {}", path.display(), e);
                    total_result.errors.push(format!("{}: {}", path.display(), e));
                    crate::logging::log_file_complete(&path.display().to_string(), 0, false);
                }
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
            check,
        )?;
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
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
    check: bool,
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
        process_bin_file(
            file,
            config,
            selected_fixes,
            champions,
            dry_run,
            hash_provider,
            check,
        )
    } else if file_name.ends_with(".wad.client") {
        process_wad_file(
            file,
            config,
            selected_fixes,
            champions,
            dry_run,
            hash_provider,
            check,
        )
    } else if ext == "fantome" || ext == "zip" {
        process_fantome_file(
            file,
            config,
            selected_fixes,
            champions,
            dry_run,
            hash_provider,
            check,
        )
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
    check: bool,
) -> Result<ProcessResult> {
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

        tracing::info!("=> Wrote fixed BIN to: {}", output_path.display());
        tracing::info!(
            "  {} fixes applied, {} bytes written",
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
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
    check: bool,
) -> Result<ProcessResult> {
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
        .filter(|(path, _)| path.to_lowercase().ends_with(".bin"))
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
    let wad_output = wad_pipeline::apply_wad_fixes(&all_files, config, selected_fixes)?;

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
            if let Some((_, bytes)) = all_files.iter_mut().find(|(p, _)| p == &conversion.path) {
                match converter_registry.convert(&conversion.converter, bytes) {
                    Ok(converted_bytes) => {
                        let old_size = bytes.len();
                        *bytes = converted_bytes;
                        conversion_count += 1;
                        tracing::info!(
                            "=> Converted {} from .{} to .{} ({} -> {} bytes)",
                            conversion.path,
                            conversion.from_ext,
                            conversion.to_ext,
                            old_size,
                            bytes.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "X Converter '{}' failed for {}: {}",
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

    for (path, bytes) in &bin_chunks {
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
        if let Some((_, bytes)) = all_files.iter().find(|(p, _)| *p == linked_path) {
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
        bin_chunks.iter().map(|(p, _)| p.clone()).collect();
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
    for (path, _) in &bin_chunks {
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
        let fixes_applied = result.fixes_applied;
        total_result.merge(result);

        // Write modified BIN back to all_files collection
        if !dry_run && fixes_applied > 0 {
            match bin_provider.write_bytes(&ctx.tree) {
                Ok(modified_bytes) => {
                    // Update the BIN bytes in all_files
                    if let Some((_, file_bytes)) = all_files.iter_mut().find(|(p, _)| p == path) {
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

    // In check mode, populate CheckInfo with skin detection
    if check {
        use hematite_core::detect::skin::SkinDetector;

        let all_paths: Vec<String> = all_files.iter().map(|(p, _)| p.clone()).collect();
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
        use league_toolkit::wad::{WadBuilder, WadChunkBuilder};
        use std::io::Write;
        use xxhash_rust::xxh64::xxh64;

        tracing::info!("Building modified WAD...");

        let mut builder = WadBuilder::default();
        let mut chunks_included = 0;

        for (path, _) in &all_files {
            if !shared_files_to_remove.contains(path) {
                builder = builder.with_chunk(WadChunkBuilder::default().with_path(path));
                chunks_included += 1;
            } else {
                tracing::debug!("Excluding removed file: {}", path);
            }
        }

        let output_path = file.with_extension("fixed.wad.client");
        let mut output_file =
            std::fs::File::create(&output_path).context("Failed to create output WAD file")?;

        builder.build_to_writer(&mut output_file, |path_hash, cursor| {
            let (path, bytes) = all_files
                .iter()
                .find(|(p, _)| xxh64(p.to_lowercase().as_bytes(), 0) == path_hash)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Missing file for hash {:016X}", path_hash),
                    )
                })?;

            tracing::trace!("Writing chunk: {} ({} bytes)", path, bytes.len());
            cursor.write_all(bytes)?;
            Ok(())
        })?;

        tracing::info!("=> Wrote fixed WAD to: {}", output_path.display());
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
    config: &FixConfig,
    selected_fixes: &[String],
    champions: &CharacterRelations,
    dry_run: bool,
    hash_provider: &Arc<dyn HashProvider>,
    check: bool,
) -> Result<ProcessResult> {
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
    let mut fixed_wad_paths = Vec::new();

    for wad_path in &wad_paths {
        let result = process_wad_file(
            wad_path,
            config,
            selected_fixes,
            champions,
            dry_run,
            hash_provider,
            check,
        )?;

        // Track the fixed WAD path (original.wad.client -> original.fixed.wad.client)
        let has_fixes = result.fixes_applied > 0;
        total_result.merge(result);

        if !dry_run && has_fixes {
            let fixed_path = wad_path.with_extension("fixed.wad.client");
            if fixed_path.exists() {
                fixed_wad_paths.push((wad_path.clone(), fixed_path));
            }
        }
    }

    // Rebuild the fantome/zip archive with fixed WAD files
    if !dry_run && !fixed_wad_paths.is_empty() && !check {
        rebuild_fantome_archive(file, &temp_dir, &fixed_wad_paths)?;
        tracing::info!(
            "=> Rebuilt fantome with {} fixed WAD file(s)",
            fixed_wad_paths.len()
        );
    }

    Ok(total_result)
}

/// Rebuild a fantome/zip archive, replacing WAD files with their fixed versions.
fn rebuild_fantome_archive(
    original_file: &Path,
    _temp_dir: &tempfile::TempDir,
    fixed_wads: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<()> {
    use std::io::{Read, Write};

    // Read the original archive to copy non-WAD files
    let original_zip = std::fs::File::open(original_file)?;
    let mut original_archive = zip::ZipArchive::new(std::io::BufReader::new(original_zip))?;

    // Create output path: original.fantome -> original.fixed.fantome
    let output_path = if original_file.extension().and_then(|e| e.to_str()) == Some("fantome") {
        original_file.with_extension("fixed.fantome")
    } else {
        original_file.with_extension("fixed.zip")
    };

    let output_file = std::fs::File::create(&output_path)?;
    let mut output_archive = zip::ZipWriter::new(output_file);

    // Create a map of original WAD paths to fixed WAD paths
    let fixed_map: std::collections::HashMap<String, &std::path::Path> = fixed_wads
        .iter()
        .map(|(orig, fixed)| {
            (
                orig.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
                    .replace(".wad.client", "")
                    + ".wad.client",
                fixed.as_path(),
            )
        })
        .collect();

    // Copy all files from original archive, replacing WADs with fixed versions
    for i in 0..original_archive.len() {
        let mut entry = original_archive.by_index(i)?;
        let entry_name = entry.name().to_string();

        // Use same compression method as original
        let options = zip::write::FileOptions::default()
            .compression_method(entry.compression())
            .unix_permissions(entry.unix_mode().unwrap_or(0o644));

        output_archive.start_file(&entry_name, options)?;

        // Check if this is a WAD file that was fixed
        let is_wad = entry_name.to_lowercase().ends_with(".wad.client");
        let fixed_wad = if is_wad {
            fixed_map.get(&entry_name.to_lowercase())
        } else {
            None
        };

        if let Some(fixed_path) = fixed_wad {
            // Write the fixed WAD instead of the original
            let fixed_data = std::fs::read(fixed_path)?;
            output_archive.write_all(&fixed_data)?;
            tracing::debug!("Replaced {} with fixed version", entry_name);
        } else {
            // Copy original file as-is
            let mut buffer = Vec::new();
            entry.read_to_end(&mut buffer)?;
            output_archive.write_all(&buffer)?;
        }
    }

    output_archive.finish()?;
    tracing::info!("=> Wrote fixed fantome to: {}", output_path.display());

    Ok(())
}
