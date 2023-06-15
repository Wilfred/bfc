//! Human-readable warnings and errors for the CLI.

use crate::bfir::Position;

#[derive(Debug, PartialEq, Eq)]
pub struct Warning {
    pub message: String,
    pub position: Option<Position>,
}
