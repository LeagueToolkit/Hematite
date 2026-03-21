//! Structured logging with tracing and colored console output.

use crate::args::Verbosity;
use colored::Colorize;
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Initialize the tracing subscriber based on verbosity level.
pub fn init(verbosity: &Verbosity, json_mode: bool) {
    let level = match verbosity {
        Verbosity::Quiet => Level::ERROR,
        Verbosity::Normal => Level::INFO,
        Verbosity::Verbose => Level::DEBUG,
        Verbosity::Trace => Level::TRACE,
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(level.into())
        .add_directive("hematite_cli=debug".parse().unwrap())
        .add_directive("hematite_core=debug".parse().unwrap())
        .add_directive("hematite_ltk=debug".parse().unwrap());

    if json_mode {
        // JSON output for automation
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        // Human-readable colored output
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_level(true)
            .init();
    }
}

/// Log a session start banner (human-readable mode only).
pub fn log_session_start(input: &str, selected_fixes: &[String]) {
    println!("{}", "═".repeat(60).bright_cyan());
    println!("{}", "  Hematite — League of Legends Skin Fixer".bright_cyan().bold());
    println!("{}", "═".repeat(60).bright_cyan());
    println!("{}: {}", "Input".bright_white().bold(), input);

    if selected_fixes.is_empty() {
        println!("{}: {}", "Mode".bright_white().bold(), "Auto-detect (all fixes)".yellow());
    } else {
        println!("{}: {} selected", "Fixes".bright_white().bold(), selected_fixes.len());
        for fix_id in selected_fixes {
            println!("  {} {}", "•".bright_cyan(), fix_id.bright_white());
        }
    }
    println!();
}

/// Log fix detection.
#[allow(dead_code)]
pub fn log_issue_detected(fix_name: &str, file_path: &str) {
    tracing::info!(
        "{} {} in {}",
        "⚠".yellow(),
        format!("Detected: {}", fix_name).yellow(),
        file_path.bright_white()
    );
}

/// Log successful fix application.
#[allow(dead_code)]
pub fn log_fix_success(fix_name: &str, changes: usize) {
    tracing::info!(
        "{} {} ({} changes)",
        "✓".green(),
        format!("Applied: {}", fix_name).green(),
        changes
    );
}

/// Log failed fix.
#[allow(dead_code)]
pub fn log_fix_failed(fix_name: &str, error: &str) {
    tracing::warn!(
        "{} {} - {}",
        "✗".red(),
        format!("Failed: {}", fix_name).red(),
        error
    );
}

/// Log session summary.
pub fn log_session_summary(result: &hematite_types::result::ProcessResult, duration: f64) {
    println!();
    println!("{}", "═".repeat(60).bright_cyan());
    println!("{}", "  Summary".bright_cyan().bold());
    println!("{}", "═".repeat(60).bright_cyan());
    println!("{}: {}", "Files processed".bright_white().bold(), result.files_processed);
    println!("{}: {}", "Fixes applied".bright_white().bold(), result.fixes_applied.to_string().green());
    println!("{}: {}", "Fixes failed".bright_white().bold(), result.fixes_failed.to_string().red());

    if !result.errors.is_empty() {
        println!("\n{}:", "Errors".red().bold());
        for error in &result.errors {
            println!("  {} {}", "•".red(), error);
        }
    }

    println!("\n{}: {:.2}s", "Duration".bright_white().bold(), duration);
    println!("{}", "═".repeat(60).bright_cyan());
}

/// Print warning message.
#[allow(dead_code)]
pub fn log_warning(msg: &str) {
    tracing::warn!("{} {}", "⚠".yellow(), msg.yellow());
}
