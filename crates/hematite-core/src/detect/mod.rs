//! Issue detection rules.
//!
//! Each [`DetectionRule`] variant maps to a detection function in [`rules`].
//! Detection is read-only — it examines the BIN tree and returns true/false.

pub mod bnk;
pub mod rules;

pub use rules::detect_issue;
