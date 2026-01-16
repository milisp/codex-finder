//! Common command utilities to prevent console windows from appearing on Windows

use std::process::{Command as StdCommand, Stdio};
use tokio::process::Command as TokioCommand;

/// Create a std Command that won't show a window on Windows
///
/// On Windows, this sets the CREATE_NO_WINDOW flag to prevent a console window
/// from appearing when running external processes.
#[cfg(windows)]
pub fn create_command(program: &str) -> StdCommand {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = StdCommand::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
pub fn create_command(program: &str) -> StdCommand {
    StdCommand::new(program)
}

/// Create a std Command with stdin set to null to prevent console windows
///
/// This is useful for commands that don't need stdin and should run silently.
#[cfg(windows)]
pub fn create_silent_command(program: &str) -> StdCommand {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = StdCommand::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdin(Stdio::null());
    cmd
}

#[cfg(not(windows))]
pub fn create_silent_command(program: &str) -> StdCommand {
    let mut cmd = StdCommand::new(program);
    cmd.stdin(Stdio::null());
    cmd
}

/// Create a tokio Command that won't show a window on Windows
///
/// On Windows, this sets the CREATE_NO_WINDOW flag to prevent a console window
/// from appearing when running external processes.
#[cfg(windows)]
pub fn create_tokio_command(program: &str) -> TokioCommand {
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = TokioCommand::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
pub fn create_tokio_command(program: &str) -> TokioCommand {
    TokioCommand::new(program)
}

/// Create a tokio Command with stdin set to null to prevent console windows
///
/// This is useful for async commands that don't need stdin and should run silently.
#[cfg(windows)]
pub fn create_tokio_silent_command(program: &str) -> TokioCommand {
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = TokioCommand::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stdin(Stdio::null());
    cmd
}

#[cfg(not(windows))]
pub fn create_tokio_silent_command(program: &str) -> TokioCommand {
    let mut cmd = TokioCommand::new(program);
    cmd.stdin(Stdio::null());
    cmd
}
