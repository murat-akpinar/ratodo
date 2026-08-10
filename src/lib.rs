//! ratodo — a todo TUI, built with ratatui. See docs/README.md.
//!
//! The pure core (`parse`, `write`, `capture`) is a library so it can be tested
//! without a terminal; the binary is a thin shell over it.

pub mod agenda;
pub mod capture;
pub mod model;
pub mod parse;
pub mod text;
pub mod write;
