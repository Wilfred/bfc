//! This module defines a convenient API for shelling out to commands,
//! handling stderr when they fail.

use std::process::Command;

/// Execute the CLI command specified.
///
/// # Failures
///
/// If the command isn't on $PATH, returns Err with a helpful
/// message. If the command returns a non-zero exit code, returns Err
/// with stderr.
pub fn run_shell_command(command: &str, args: &[&str]) -> Result<(), String> {
    let mut c = Command::new(command);
    for arg in args {
        c.arg(arg);
    }

    match c.output() {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                Err((*stderr).to_owned())
            }
        }
        Err(_) => Err(format!("Could not execute '{}'. Is it on $PATH?", command)),
    }
}
