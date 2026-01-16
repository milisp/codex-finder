//! WSL (Windows Subsystem for Linux) support utilities
//!
//! This module provides functions for:
//! - Detecting and listing WSL distributions
//! - Converting between Windows, WSL, and UNC paths
//! - Executing commands in WSL
//! - Reading files from WSL filesystem
//! - Discovering WSL home directories and project roots

use anyhow::Result;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use super::command::create_silent_command;

#[cfg(windows)]
use anyhow::Context;
#[cfg(windows)]
use regex::Regex;

/// Represents a WSL distribution
#[derive(Debug, Clone)]
pub struct WslDistro {
    /// Distribution name (e.g., "Ubuntu-24.04")
    pub name: String,
    /// Whether this is the default distribution
    pub is_default: bool,
    /// Current state (e.g., "Running", "Stopped")
    pub state: String,
    /// WSL version (1 or 2)
    pub version: u8,
}

/// Check if we're running on Windows
#[cfg(windows)]
pub fn is_windows() -> bool {
    true
}

#[cfg(not(windows))]
pub fn is_windows() -> bool {
    false
}

/// List all available WSL distributions
#[cfg(windows)]
pub fn list_distros() -> Result<Vec<WslDistro>> {
    let output = create_silent_command("wsl.exe")
        .args(["-l", "-v"])
        .output()
        .context("Failed to execute wsl.exe -l -v")?;

    if !output.status.success() {
        anyhow::bail!("wsl.exe -l -v failed");
    }

    parse_distro_list(&output.stdout)
}

#[cfg(not(windows))]
pub fn list_distros() -> Result<Vec<WslDistro>> {
    Ok(Vec::new())
}

/// Parse the output of `wsl.exe -l -v`
#[cfg(windows)]
fn parse_distro_list(raw_output: &[u8]) -> Result<Vec<WslDistro>> {
    use encoding_rs::*;

    // Try UTF-16LE first (Windows default), then UTF-8
    let (text, _, _) = UTF_16LE.decode(raw_output);
    let text = text.to_string();

    // Remove ANSI escape sequences
    let ansi_regex = Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").unwrap();
    let text = ansi_regex.replace_all(&text, "");

    // Remove control characters
    let control_regex = Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap();
    let text = control_regex.replace_all(&text, "");

    let mut distros = Vec::new();

    for line in text.lines().skip(1) {
        // Skip header line
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse format: "  NAME                   STATE           VERSION"
        // or: "* NAME                   STATE           VERSION" (default marked with *)
        let is_default = line.starts_with('*');
        let line = line.trim_start_matches('*').trim();

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let state = parts[1].to_string();
            let version = parts[2].parse::<u8>().unwrap_or(2);

            distros.push(WslDistro {
                name,
                is_default,
                state,
                version,
            });
        }
    }

    Ok(distros)
}

/// Check if a specific distribution exists
#[cfg(windows)]
pub fn distro_exists(name: &str) -> bool {
    list_distros()
        .map(|distros| distros.iter().any(|d| d.name == name))
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn distro_exists(_name: &str) -> bool {
    false
}

/// Get the default or preferred WSL distribution
#[cfg(windows)]
pub fn get_preferred_distro(preferred: Option<&str>) -> Option<String> {
    let distros = list_distros().ok()?;

    if distros.is_empty() {
        return None;
    }

    // If preferred is specified and exists, use it
    if let Some(pref) = preferred 
        && distros.iter().any(|d| d.name == pref) {
        return Some(pref.to_string());
    }

    // Priority order:
    // 1. Ubuntu-24.04
    // 2. Ubuntu-22.04
    // 3. Ubuntu
    // 4. Default distro (marked with *)
    // 5. First running distro
    // 6. First available distro

    let ubuntu_versions = ["Ubuntu-24.04", "Ubuntu-22.04", "Ubuntu"];
    for version in &ubuntu_versions {
        if distros.iter().any(|d| d.name == *version) {
            return Some(version.to_string());
        }
    }

    if let Some(default) = distros.iter().find(|d| d.is_default) {
        return Some(default.name.clone());
    }

    if let Some(running) = distros.iter().find(|d| d.state == "Running") {
        return Some(running.name.clone());
    }

    distros.first().map(|d| d.name.clone())
}

#[cfg(not(windows))]
pub fn get_preferred_distro(_preferred: Option<&str>) -> Option<String> {
    None
}

/// Convert Windows path to WSL path
/// Example: C:\Users\name\file.txt -> /mnt/c/Users/name/file.txt
#[cfg(windows)]
pub fn win_to_wsl<P: AsRef<Path>>(win_path: P, distro: Option<&str>) -> Result<String> {
    let path_str = win_path.as_ref().to_string_lossy();

    // Try using wslpath command first
    if let Some(distro_name) = distro
        && let Ok(output) = create_silent_command("wsl.exe")
            .args(["-d", distro_name, "wslpath", "-a", &path_str])
            .output()
        && output.status.success() {
        let wsl_path = String::from_utf8_lossy(&output.stdout);
        return Ok(wsl_path.trim().to_string());
    }

    // Fallback: manual conversion
    // C:\Users\... -> /mnt/c/Users/...
    let path_str = path_str.replace('\\', "/");

    if let Some(drive_letter) = path_str.chars().next()
        && path_str.len() > 2
        && path_str.chars().nth(1) == Some(':') {
        let drive = drive_letter.to_lowercase();
        let rest = &path_str[2..];
        return Ok(format!("/mnt/{}{}", drive, rest));
    }

    Ok(path_str.to_string())
}

#[cfg(not(windows))]
pub fn win_to_wsl<P: AsRef<Path>>(win_path: P, _distro: Option<&str>) -> Result<String> {
    Ok(win_path.as_ref().to_string_lossy().to_string())
}

/// Convert WSL path to Windows UNC path
/// Example: /home/user/.codex -> \\wsl.localhost\Ubuntu-24.04\home\user\.codex
#[cfg(windows)]
pub fn wsl_to_unc<P: AsRef<Path>>(wsl_path: P, distro: &str) -> String {
    let path = wsl_path.as_ref().to_string_lossy();
    let path = path.trim_start_matches('/');
    format!("\\\\wsl.localhost\\{}\\{}", distro, path)
}

#[cfg(not(windows))]
pub fn wsl_to_unc<P: AsRef<Path>>(wsl_path: P, _distro: &str) -> String {
    wsl_path.as_ref().to_string_lossy().to_string()
}

/// Convert UNC path to WSL path
/// Example: \\wsl.localhost\Ubuntu-24.04\home\user\.codex -> /home/user/.codex
#[cfg(windows)]
pub fn unc_to_wsl<P: AsRef<Path>>(unc_path: P) -> Option<(String, String)> {
    let path = unc_path.as_ref().to_string_lossy();

    // Match both \\wsl.localhost\ and \\wsl$\ formats
    let unc_regex = Regex::new(r"^\\\\wsl(?:\.|\.localhost)[\\/]([^\\]+)[\\/](.*)$").unwrap();

    if let Some(caps) = unc_regex.captures(&path) {
        let distro = caps.get(1)?.as_str().to_string();
        let wsl_path = caps.get(2)?.as_str().replace('\\', "/");
        return Some((distro, format!("/{}", wsl_path)));
    }

    None
}

#[cfg(not(windows))]
pub fn unc_to_wsl<P: AsRef<Path>>(_unc_path: P) -> Option<(String, String)> {
    None
}

/// Check if a path is a UNC path to WSL
#[cfg(windows)]
pub fn is_unc_path<P: AsRef<Path>>(path: P) -> bool {
    let path_str = path.as_ref().to_string_lossy();
    path_str.starts_with("\\\\wsl.localhost\\") || path_str.starts_with("\\\\wsl$\\")
}

#[cfg(not(windows))]
pub fn is_unc_path<P: AsRef<Path>>(_path: P) -> bool {
    false
}

/// Normalize UNC path to use wsl.localhost format
#[cfg(windows)]
pub fn normalize_unc<P: AsRef<Path>>(path: P) -> String {
    let path_str = path.as_ref().to_string_lossy();
    path_str.replace("\\\\wsl$\\", "\\\\wsl.localhost\\")
}

#[cfg(not(windows))]
pub fn normalize_unc<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_string_lossy().to_string()
}

/// Execute a command in WSL
#[cfg(windows)]
pub fn exec_in_wsl(distro: &str, cmd: &str) -> Result<String> {
    let output = create_silent_command("wsl.exe")
        .args(["-d", distro, "--", "sh", "-c", cmd])
        .output()
        .context("Failed to execute command in WSL")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("WSL command failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(not(windows))]
pub fn exec_in_wsl(_distro: &str, _cmd: &str) -> Result<String> {
    anyhow::bail!("WSL is only available on Windows")
}

/// Read a file from WSL filesystem
#[cfg(windows)]
pub fn read_file_in_wsl(distro: &str, wsl_path: &str) -> Result<String> {
    exec_in_wsl(distro, &format!("cat '{}'", wsl_path))
}

#[cfg(not(windows))]
pub fn read_file_in_wsl(_distro: &str, _wsl_path: &str) -> Result<String> {
    anyhow::bail!("WSL is only available on Windows")
}

/// Get the home directory for a WSL distribution
#[cfg(windows)]
pub fn get_distro_home(distro: &str) -> Result<String> {
    exec_in_wsl(distro, "echo $HOME")
}

#[cfg(not(windows))]
pub fn get_distro_home(_distro: &str) -> Result<String> {
    anyhow::bail!("WSL is only available on Windows")
}

/// Get UNC path to .codex directory in a WSL distro
#[cfg(windows)]
pub fn get_distro_codex_unc(distro: &str) -> Result<String> {
    let home = get_distro_home(distro)?;
    let codex_path = format!("{}/.codex", home);
    Ok(wsl_to_unc(&codex_path, distro))
}

#[cfg(not(windows))]
pub fn get_distro_codex_unc(_distro: &str) -> Result<String> {
    anyhow::bail!("WSL is only available on Windows")
}

/// Get all potential codex root directories from WSL distributions
#[cfg(windows)]
pub fn get_wsl_codex_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(distros) = list_distros() {
        for distro in distros {
            // Try to get .codex directory
            if let Ok(codex_unc) = get_distro_codex_unc(&distro.name) {
                roots.push(PathBuf::from(codex_unc));
            }

            // Try to get home directory
            if let Ok(home) = get_distro_home(&distro.name) {
                roots.push(PathBuf::from(wsl_to_unc(&home, &distro.name)));

                // Also add ~/code if it exists
                let code_path = format!("{}/code", home);
                roots.push(PathBuf::from(wsl_to_unc(&code_path, &distro.name)));
            }
        }
    }

    roots
}

#[cfg(not(windows))]
pub fn get_wsl_codex_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_win_to_wsl_conversion() {
        let result = win_to_wsl("C:\\Users\\test\\file.txt", None);
        assert!(result.is_ok());
        let wsl_path = result.unwrap();
        assert!(wsl_path.contains("/mnt/c"));
    }

    #[test]
    #[cfg(windows)]
    fn test_wsl_to_unc_conversion() {
        let unc = wsl_to_unc("/home/user/.codex", "Ubuntu-24.04");
        assert_eq!(unc, "\\\\wsl.localhost\\Ubuntu-24.04\\home\\user\\.codex");
    }

    #[test]
    #[cfg(windows)]
    fn test_unc_to_wsl_conversion() {
        let result = unc_to_wsl("\\\\wsl.localhost\\Ubuntu-24.04\\home\\user\\.codex");
        assert!(result.is_some());
        let (distro, path) = result.unwrap();
        assert_eq!(distro, "Ubuntu-24.04");
        assert_eq!(path, "/home/user/.codex");
    }

    #[test]
    #[cfg(windows)]
    fn test_is_unc_path() {
        assert!(is_unc_path("\\\\wsl.localhost\\Ubuntu\\home"));
        assert!(is_unc_path("\\\\wsl$\\Ubuntu\\home"));
        assert!(!is_unc_path("C:\\Users\\test"));
    }

    #[test]
    #[cfg(windows)]
    fn test_normalize_unc() {
        let normalized = normalize_unc("\\\\wsl$\\Ubuntu\\home");
        assert_eq!(normalized, "\\\\wsl.localhost\\Ubuntu\\home");
    }
}
