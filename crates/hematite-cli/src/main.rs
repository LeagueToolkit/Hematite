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

fn main() {
    // TODO: Implement CLI entry point
    //
    // 1. Parse CLI args
    // 2. Initialize logging
    // 3. Load hash provider (TxtHashProvider or LmdbHashProvider)
    // 4. Load config (fetch from GitHub with cache, or embedded fallback)
    // 5. Load champion list (same fetch pattern)
    // 6. Route to processor based on input type
    // 7. Output results (JSON or human-readable)
    // 8. Exit with appropriate code

    use clap::Parser;
    let cli = args::Cli::parse();
    logging::init(&cli.verbosity);
    let _selected = args::collect_selected_fixes(&cli);

    println!("hematite-cli v{}", env!("CARGO_PKG_VERSION"));
    println!("Input: {:?}", cli.input);
    println!("TODO: Implementation pending LTK rewrite");
}
