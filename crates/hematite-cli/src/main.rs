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

    // Validate input exists
    if !cli.input.exists() {
        anyhow::bail!("Input path does not exist: {}", cli.input.display());
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
        logging::log_session_start(&cli.input.to_string_lossy(), &selected_fixes);
    }

    // In check mode, force dry_run
    let dry_run = cli.dry_run || cli.check;

    // Build repath options (only active when --repath is passed)
    let repath_opts: Option<RepathOptions> = if cli.repath {
        let prefix = cli
            .repath_prefix
            .clone()
            .unwrap_or_else(|| "bum".to_string());
        let mut opts = RepathOptions::new(prefix);
        opts.invis_texture = cli.invis_texture;
        Some(opts)
    } else {
        None
    };

    // Process input
    let result = process::process_input(
        &cli.input,
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
