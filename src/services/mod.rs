pub mod cups;
pub mod sane;

pub mod escputil;

use std::process::Command;

/// Helper function to check if a command exists
pub fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}