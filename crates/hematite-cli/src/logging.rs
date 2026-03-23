//! Structured logging with tracing and colored console output.

use crate::args::Verbosity;
use colored::Colorize;
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Initialize the tracing subscriber based on verbosity level.
pub fn init(verbosity: &Verbosity, json_mode: bool) {
    // Enable ANSI color support on Windows
    #[cfg(windows)]
    {
        let _ = colored::control::set_virtual_terminal(true);
    }

    let filter = match verbosity {
        Verbosity::Quiet => {
            // Only errors
            EnvFilter::from_default_env()
                .add_directive(Level::ERROR.into())
        }
        Verbosity::Normal => {
            // INFO and above, no debug spam
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
        }
        Verbosity::Verbose => {
            // DEBUG level for hematite crates
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("hematite_cli=debug".parse().unwrap())
                .add_directive("hematite_core=debug".parse().unwrap())
                .add_directive("hematite_ltk=debug".parse().unwrap())
        }
        Verbosity::Trace => {
            // TRACE everything
            EnvFilter::from_default_env()
                .add_directive(Level::TRACE.into())
                .add_directive("hematite_cli=trace".parse().unwrap())
                .add_directive("hematite_core=trace".parse().unwrap())
                .add_directive("hematite_ltk=trace".parse().unwrap())
        }
    };

    if json_mode {
        // JSON output for automation
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        // Human-readable colored output (hide timestamps in normal mode)
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_level(matches!(verbosity, Verbosity::Verbose | Verbosity::Trace))
            .without_time()
            .init();
    }
}

/// Log a session start banner (human-readable mode only).
pub fn log_session_start(input: &str, selected_fixes: &[String]) {
    println!();
    println!("{}", "=".repeat(70).bright_cyan());
    println!(
        "{}",
        "  Hematite - League of Legends Skin Fixer"
            .bright_cyan()
            .bold()
    );
    println!("{}", "=".repeat(70).bright_cyan());
    println!();
    println!("  {}: {}", "Input".bright_white().bold(), input.bright_yellow());

    if selected_fixes.is_empty() {
        println!("  {}: {}", "Mode".bright_white().bold(), "Auto-detect (all fixes)".green());
    } else {
        println!(
            "  {}: {} selected",
            "Fixes".bright_white().bold(),
            selected_fixes.len().to_string().cyan()
        );
        for fix_id in selected_fixes {
            println!("    {} {}", ">".bright_cyan(), fix_id.bright_white());
        }
    }
    println!();
    println!("{}", "=".repeat(70).bright_cyan());
    println!();
}

/// Log session summary.
pub fn log_session_summary(result: &hematite_types::result::ProcessResult, duration: f64) {
    println!();
    println!("{}", "=".repeat(70).bright_cyan());
    println!("{}", "  Summary".bright_cyan().bold());
    println!("{}", "=".repeat(70).bright_cyan());
    println!();
    println!(
        "  {}: {}",
        "Files processed".bright_white().bold(),
        result.files_processed.to_string().cyan()
    );
    println!(
        "  {}: {}",
        "Fixes applied".bright_white().bold(),
        result.fixes_applied.to_string().green().bold()
    );

    if result.fixes_failed > 0 {
        println!(
            "  {}: {}",
            "Fixes failed".bright_white().bold(),
            result.fixes_failed.to_string().red().bold()
        );
    }

    if !result.errors.is_empty() {
        println!();
        println!("  {}:", "Errors".red().bold());
        for error in &result.errors {
            println!("    {} {}", "X".red().bold(), error);
        }
    }

    println!();
    println!("  {}: {:.2}s", "Duration".bright_white().bold(), duration.to_string().yellow());
    println!();
    println!("{}", "=".repeat(70).bright_cyan());
    println!();

    // Final status message
    if result.errors.is_empty() && result.fixes_applied > 0 {
        println!("{}", "  Success! All fixes applied.".green().bold());
    } else if result.errors.is_empty() {
        println!("{}", "  Complete! No issues detected.".green().bold());
    } else {
        println!("{}", "  Completed with errors.".yellow().bold());
    }
    println!();
}

/// Log check-mode summary (human-readable).
pub fn log_check_summary(result: &hematite_types::result::ProcessResult) {
    println!();
    println!("{}", "=".repeat(70).bright_cyan());
    println!("{}", "  Check Mode Results".bright_cyan().bold());
    println!("{}", "=".repeat(70).bright_cyan());
    println!();

    if let Some(info) = &result.check_info {
        println!(
            "  {}: {}",
            "Champion".bright_white().bold(),
            info.champion.as_deref().unwrap_or("unknown").yellow().bold()
        );
        println!(
            "  {}: {}",
            "Skin Number".bright_white().bold(),
            info.skin_number
                .map(|n| n.to_string())
                .unwrap_or_else(|| "none".to_string())
                .yellow()
                .bold()
        );
        println!(
            "  {}: {}",
            "Binless Mod".bright_white().bold(),
            if info.is_binless {
                "yes".red().bold()
            } else {
                "no".green().bold()
            }
        );

        println!();

        if info.detected_issues.is_empty() {
            println!(
                "  {}",
                "No issues detected - mod looks clean!".green().bold()
            );
        } else {
            println!(
                "  {} ({}):",
                "Detected Issues".red().bold(),
                info.detected_issues.len()
            );
            println!();
            for issue in &info.detected_issues {
                println!("    {} {}", ">".red().bold(), issue.bright_white());
            }
        }
    } else {
        println!("  {}", "No check info available".yellow());
    }

    println!();
    println!("{}", "=".repeat(70).bright_cyan());
    println!();
}

/// Log batch processing start.
pub fn log_batch_start(count: usize) {
    println!();
    println!(
        "  {} {}",
        "Found".bright_white(),
        format!("{} file(s) to process", count).cyan().bold()
    );
    println!();
}

/// Log individual file processing in batch mode.
pub fn log_file_progress(current: usize, total: usize, path: &str) {
    let progress = format!("[{}/{}]", current, total);
    println!(
        "  {} {} {}",
        progress.bright_cyan().bold(),
        "Processing:".bright_white(),
        path.yellow()
    );
}

/// Log file completion in batch mode.
pub fn log_file_complete(path: &str, fixes_applied: u32, success: bool) {
    if success {
        if fixes_applied > 0 {
            println!(
                "    {} {} ({} fixes applied)",
                "OK".green().bold(),
                path.bright_white(),
                fixes_applied.to_string().green()
            );
        } else {
            println!(
                "    {} {} (no issues found)",
                "OK".green().bold(),
                path.bright_white()
            );
        }
    } else {
        println!(
            "    {} {}",
            "FAILED".red().bold(),
            path.bright_white()
        );
    }
}
