//! This module defines a convenient API for shelling out to commands,
//! handling stderr when they fail.

use std::process::Command;

/// Execute the CLI command specified. If the command succeeds,
/// returns stdout.
///
/// # Failures
///
/// If the command isn't on $PATH, returns Err with a helpful
/// message. If the command returns a non-zero exit code, returns Err
/// with stderr.
fn shell_command(command: &str, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new(command);
    for arg in args {
        c.arg(arg);
    }

    match c.output() {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                Ok((*stdout).to_owned())
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                Err((*stderr).to_owned())
            }
        }
        Err(_) => Err(format!("Could not execute '{}'. Is it on $PATH?", command)),
    }
}

/// Execute a CLI command as `shell_command`, but ignore stdout.
pub fn run_shell_command(command: &str, args: &[&str]) -> Result<(), String> {
    match shell_command(command, args) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
