//! Hematite CLI — League of Legends custom skin fixer.
//!
//! ## Usage
//! ```bash
//! hematite-cli "path/to/skin.fantome"              # Auto-detect and fix all
//! hematite-cli "skin.fantome" --all                 # Apply all fixes
//! hematite-cli "skin.fantome" --healthbar --vfx-shape  # Specific fixes
//! hematite-cli "skin.fantome" --dry-run             # Show what would be fixed
//! hematite-cli "skin.fantome" --json                # JSON output
//! hematite-cli "path/to/skins_folder/"              # Batch directory
//! ```

mod args;
mod hash_downloader;
mod logging;
mod process;
mod remote;
mod version_check;

use anyhow::Result;
use clap::Parser;
use hematite_types::champion::CharacterRelations;
use hematite_types::repath::RepathOptions;
use std::time::Instant;

fn main() {
    let result = run();

    if let Err(ref e) = result {
        eprintln!("Error: {e:#}");
    }

    // Pause before exit so console doesn't close instantly when double-clicked
    if !std::env::args().any(|a| a == "--json" || a == "--no-pause") {
        eprintln!();
        eprintln!("Press Enter to exit...");
        let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
    }

    if result.is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Parse CLI arguments
    let cli = args::Cli::parse();

    // Initialize logging
    logging::init(&cli.verbosity, cli.json);

    // -- Version gate -------------------------------------------------------
    // Runs before input validation so `--check-version` works without
    // requiring an input path. JSON-mode callers shouldn't see human-
    // readable banners; the hard-block still fires but silently.
    let version_outcome = version_check::check_version();
    if cli.check_version {
        // Force a banner even in --json mode so the user gets feedback,
        // then exit cleanly regardless of gate status.
        let blocked = version_check::report(&version_outcome, true);
        // `report` only prints when there's something to say; explicit
        // "you're good" line so `--check-version` always gives feedback.
        if !blocked
            && matches!(
                version_outcome.status,
                version_check::VersionStatus::UpToDate
                    | version_check::VersionStatus::Unknown
            )
        {
            eprintln!(
                "Hematite-CLI {} — up to date.",
                env!("CARGO_PKG_VERSION")
            );
        }
        if blocked {
            std::process::exit(2);
        }
        return Ok(());
    }
    if !cli.json {
        let blocked = version_check::report(&version_outcome, cli.skip_version_check);
        if blocked {
            anyhow::bail!(
                "Refusing to run: CLI is older than the published minimum. \
                 Pass --skip-version-check to override."
            );
        }
    } else if matches!(version_outcome.status, version_check::VersionStatus::Outdated { .. })
        && !cli.skip_version_check
    {
        anyhow::bail!(
            "Refusing to run: CLI is older than the published minimum. \
             Pass --skip-version-check to override (see --check-version)."
        );
    }

    // After `--check-version` short-circuit, `input` is guaranteed present
    // by clap's `required_unless_present`. Resolve it once for the rest of run().
    let input = cli
        .input
        .as_ref()
        .expect("clap should have required `input` unless --check-version was passed");

    // Validate input exists
    if !input.exists() {
        anyhow::bail!("Input path does not exist: {}", input.display());
    }

    // Start timer
    let start_time = Instant::now();

    // Collect selected fixes
    let selected_fixes = args::collect_selected_fixes(&cli);

    // Load fix configuration and champion list (tries remote, falls back to embedded)
    let config = remote::load_fix_config();
    let champion_list = remote::load_champion_list();
    let champions = CharacterRelations::from_champion_list(&champion_list);

    // Log session start (unless in JSON mode)
    if !cli.json {
        logging::log_session_start(&input.to_string_lossy(), &selected_fixes);
    }

    // In check mode, force dry_run
    let dry_run = cli.dry_run || cli.check;

    // Build repath options.
    // Priority: CLI flags > fix_config.json repath section.
    // --repath flag or config.repath.enabled activates repathing.
    let repath_opts: Option<RepathOptions> = {
        let cfg = &config.repath;
        let active = cli.repath || cfg.enabled;
        if active {
            // Pick a prefix: explicit CLI > config > Topaz-derived from filename.
            // The derived form is .{shortChar}{skinNo}_ — we make a best-effort
            // guess from the input filename: alphabetic prefix → champion,
            // first run of digits → skin number.
            let prefix = cli
                .repath_prefix
                .clone()
                .or_else(|| {
                    // Only fall back to config.prefix if it's not the legacy
                    // hard-coded default — those are clearly placeholders.
                    let p = &cfg.prefix;
                    if p.is_empty() || p == "bum" || p == "hematite" {
                        None
                    } else {
                        Some(p.clone())
                    }
                })
                .unwrap_or_else(|| derive_prefix_from_input(input));
            let mut opts = RepathOptions::new(prefix);
            opts.layout = cli.repath_layout.into();
            opts.invis_texture = cli.invis_texture || cfg.invis_texture;
            opts.skip_vo = cfg.skip_vo;
            opts.game_wad = cli.game_wad.clone();
            Some(opts)
        } else {
            None
        }
    };

    // Process input
    let result = process::process_input(
        input,
        &config,
        &selected_fixes,
        &champions,
        dry_run,
        cli.check,
        repath_opts.as_ref(),
    )?;

    // Calculate duration
    let duration = start_time.elapsed().as_secs_f64();

    // Output results
    if cli.check {
        if cli.json {
            output_check_json(&result)?;
        } else {
            logging::log_check_summary(&result);
        }
    } else if cli.json {
        output_json(&result, duration)?;
    } else {
        logging::log_session_summary(&result, duration);
    }

    // Exit with appropriate code
    if result.errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("Processing completed with {} error(s)", result.errors.len());
    }
}

/// Best-effort Topaz-style prefix from an input filename like
/// "Sasuke by Noxli (V1.0).fantome" or "ahri_skin5.zip".
///
/// Picks the first alphabetic run as the "champion" and the first digit run
/// after it as the skin number.  Falls back to "bum" if neither is found —
/// `RepathOptions::derive_prefix` already does the rest.
fn derive_prefix_from_input(input: &std::path::Path) -> String {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mod");
    let mut champion = String::new();
    for c in stem.chars() {
        if c.is_ascii_alphabetic() {
            champion.push(c);
        } else if !champion.is_empty() {
            break;
        }
    }
    let after_champ: String = stem
        .chars()
        .skip(champion.len())
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let skin_no: u32 = after_champ.parse().unwrap_or(0);
    RepathOptions::derive_prefix(&champion, skin_no)
}

/// Output check-mode results as JSON.
fn output_check_json(result: &hematite_types::result::ProcessResult) -> Result<()> {
    if let Some(check_info) = &result.check_info {
        let json = serde_json::to_string_pretty(check_info)?;
        println!("{}", json);
    } else {
        println!("{{}}");
    }
    Ok(())
}

/// Output results as JSON for automation.
fn output_json(result: &hematite_types::result::ProcessResult, duration: f64) -> Result<()> {
    #[derive(serde::Serialize)]
    struct JsonOutput {
        success: bool,
        files_processed: u32,
        fixes_applied: u32,
        fixes_failed: u32,
        errors: Vec<String>,
        duration_seconds: f64,
    }

    let output = JsonOutput {
        success: result.errors.is_empty(),
        files_processed: result.files_processed,
        fixes_applied: result.fixes_applied,
        fixes_failed: result.fixes_failed,
        errors: result.errors.clone(),
        duration_seconds: duration,
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);

    Ok(())
}
