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
mod logging;
mod process;

use anyhow::{Context, Result};
use clap::Parser;
use hematite_types::champion::{ChampionList, CharacterRelations};
use hematite_types::config::FixConfig;
use std::time::Instant;

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = args::Cli::parse();

    // Initialize logging
    logging::init(&cli.verbosity, cli.json);

    // Validate input exists
    if !cli.input.exists() {
        anyhow::bail!("Input path does not exist: {}", cli.input.display());
    }

    // Start timer
    let start_time = Instant::now();

    // Collect selected fixes
    let selected_fixes = args::collect_selected_fixes(&cli);

    // Load fix configuration and champion list
    let config = load_config()?;
    let champions = load_champions();

    // Log session start (unless in JSON mode)
    if !cli.json {
        logging::log_session_start(
            &cli.input.to_string_lossy(),
            &selected_fixes,
        );
    }

    // Process input
    let result = process::process_input(
        &cli.input,
        &config,
        &selected_fixes,
        &champions,
        cli.dry_run,
    )?;

    // Calculate duration
    let duration = start_time.elapsed().as_secs_f64();

    // Output results
    if cli.json {
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

/// Load fix configuration.
///
/// For v2 MVP, we use an embedded config. Later versions will add:
/// - Remote fetching from GitHub
/// - Local cache with TTL
/// - Fallback chain: remote → cache → embedded
fn load_config() -> Result<FixConfig> {
    const EMBEDDED_CONFIG: &str = include_str!("../../../config/fix_config.json");

    serde_json::from_str(EMBEDDED_CONFIG)
        .context("Failed to parse embedded fix configuration")
}

/// Load champion list and build character relations.
///
/// Falls back to empty relations if parsing fails (non-fatal).
fn load_champions() -> CharacterRelations {
    const EMBEDDED_CHAMPIONS: &str = include_str!("../../../config/champion_list.json");

    match serde_json::from_str::<ChampionList>(EMBEDDED_CHAMPIONS) {
        Ok(list) => CharacterRelations::from_champion_list(&list),
        Err(e) => {
            tracing::warn!("Failed to parse champion list, using empty defaults: {e}");
            CharacterRelations::default()
        }
    }
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
