//! Big "HEMATITE" splash banner printed when the CLI is invoked in
//! interactive / drag-drop mode.
//!
//! Uses ANSI 256-colour escapes via the `colored` crate (already a CLI
//! dep). Windows callers go through `colored::control::set_virtual_terminal`
//! in [`crate::logging`] so the escapes render correctly on cmd.exe.

use colored::Colorize;

/// Block-letter "HEMATITE" — generated once, embedded verbatim. Width
/// is 65 chars so it fits in an 80-col terminal with margin to spare.
const BANNER: &str = r#"
 ██╗  ██╗███████╗███╗   ███╗ █████╗ ████████╗██╗████████╗███████╗
 ██║  ██║██╔════╝████╗ ████║██╔══██╗╚══██╔══╝██║╚══██╔══╝██╔════╝
 ███████║█████╗  ██╔████╔██║███████║   ██║   ██║   ██║   █████╗
 ██╔══██║██╔══╝  ██║╚██╔╝██║██╔══██║   ██║   ██║   ██║   ██╔══╝
 ██║  ██║███████╗██║ ╚═╝ ██║██║  ██║   ██║   ██║   ██║   ███████╗
 ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝   ╚═╝   ╚══════╝
"#;

const TAGLINE: &str = "League of Legends custom-skin fixer";

/// Print the splash to stderr (so it doesn't pollute JSON / piped stdout).
pub fn print() {
    // Red on dark — matches the hematite/iron-ore colour scheme used in
    // the README banner.
    eprintln!("{}", BANNER.bright_red().bold());
    eprintln!(
        "  {}    {}",
        TAGLINE.bright_white(),
        format!("v{}", env!("CARGO_PKG_VERSION"))
            .bright_black()
    );
    eprintln!(
        "  {} {}",
        "tip:".bright_black(),
        "drag a mod onto this exe to fix it instantly"
            .bright_black()
            .italic()
    );
    eprintln!();
}

/// Print a slim divider — useful between the banner and a prompt, or
/// between two prompts.
pub fn divider() {
    eprintln!("  {}", "─".repeat(64).bright_black());
}
