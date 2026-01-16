use std::path::PathBuf;

#[cfg(windows)]
use super::wsl;

fn get_platform_binary_name() -> &'static str {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "aarch64") => "codex-aarch64-apple-darwin",
        ("macos", "x86_64") => "codex-x86_64-apple-darwin",
        ("linux", "x86_64") => "codex-x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "codex-aarch64-unknown-linux-musl",
        ("windows", "x86_64") => "codex-x86_64-pc-windows-msvc.exe",
        _ => "codex", // fallback
    }
}

fn get_vendor_platform_dir() -> Option<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-musl"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        ("windows", "aarch64") => Some("aarch64-pc-windows-msvc"),
        _ => None,
    }
}

fn get_vendor_binary_name() -> &'static str {
    if cfg!(windows) { "codex.exe" } else { "codex" }
}

pub fn discover_codex_command() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_default()
    } else {
        std::env::var("HOME").unwrap_or_default()
    };
    let binary_name = get_platform_binary_name();

    // 0) Optional override via environment variable
    if let Ok(explicit) = std::env::var("CODEX_PATH") {
        let p = PathBuf::from(&explicit);
        if p.exists() {
            log::debug!("Using CODEX_PATH override at {}", p.display());
            return Some(p);
        } else {
            log::warn!("CODEX_PATH provided but not found: {}", explicit);
        }
    }

    // First priority: Check actual binary locations in node_modules
    let binary_locations = [
        // Bun global installation
        PathBuf::from(&home)
            .join(".bun/install/global/node_modules/@openai/codex/bin")
            .join(binary_name),
        // NPM rootless (user) global installation
        PathBuf::from(&home)
            .join(".local/share/npm/lib/node_modules/@openai/codex/bin")
            .join(binary_name),
        // NPM global installations
        PathBuf::from("/usr/local/lib/node_modules/@openai/codex/bin").join(binary_name),
        PathBuf::from("/opt/homebrew/lib/node_modules/@openai/codex/bin").join(binary_name),
    ];

    for path_buf in &binary_locations {
        if path_buf.exists() {
            log::debug!("Found codex binary at {}", path_buf.display());
            return Some(path_buf.clone());
        }
    }

    if let Some(vendor_dir) = get_vendor_platform_dir() {
        let vendor_binary = get_vendor_binary_name();
        let vendor_locations = [
            PathBuf::from(&home)
                .join(".bun/install/global/node_modules/@openai/codex/vendor")
                .join(vendor_dir)
                .join("codex")
                .join(vendor_binary),
            PathBuf::from(&home)
                .join(".local/share/npm/lib/node_modules/@openai/codex/vendor")
                .join(vendor_dir)
                .join("codex")
                .join(vendor_binary),
            PathBuf::from("/usr/local/lib/node_modules/@openai/codex/vendor")
                .join(vendor_dir)
                .join("codex")
                .join(vendor_binary),
            PathBuf::from("/opt/homebrew/lib/node_modules/@openai/codex/vendor")
                .join(vendor_dir)
                .join("codex")
                .join(vendor_binary),
        ];

        for path_buf in &vendor_locations {
            if path_buf.exists() {
                log::debug!("Found codex vendor binary at {}", path_buf.display());
                return Some(path_buf.clone());
            }
        }
    }

    // Windows npm global installation paths
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let npm_paths = [
                PathBuf::from(&appdata).join("npm").join("codex.cmd"),
                PathBuf::from(&appdata).join("npm").join("codex.ps1"),
                PathBuf::from(&appdata).join("npm").join("codex"),
            ];
            for path_buf in &npm_paths {
                if path_buf.exists() {
                    log::debug!("Found npm codex at {}", path_buf.display());
                    return Some(path_buf.clone());
                }
            }
        }
    }

    // Second priority: Check if there are native rust/cargo installations
    let native_paths = [
        PathBuf::from(&home).join(".cargo/bin/codex"),
        PathBuf::from(&home).join(".cargo/bin/codex.exe"),
        PathBuf::from("/usr/local/bin/codex"),
        PathBuf::from("/opt/homebrew/bin/codex"),
    ];

    for path_buf in &native_paths {
        if path_buf.exists() {
            // Check if it's a real binary (not a js wrapper)
            if let Ok(content) = std::fs::read_to_string(path_buf) {
                if content.contains("codex.js") || content.starts_with("#!/usr/bin/env node") {
                    // This is a wrapper script, skip it
                    log::debug!("Skipping wrapper script at {}", path_buf.display());
                    continue;
                }
            }
            log::debug!("Found native codex binary at {}", path_buf.display());
            return Some(path_buf.clone());
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        let mut wrapper_candidate: Option<PathBuf> = None;
        let candidate_names: &[&str] = if cfg!(windows) {
            &["codex.exe", "codex.cmd", "codex.ps1", "codex"]
        } else {
            &["codex"]
        };
        for dir in path_env.split(separator) {
            if dir.is_empty() {
                continue;
            }
            for name in candidate_names {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.exists() {
                    if let Ok(content) = std::fs::read_to_string(&candidate) {
                        let is_wrapper = content.contains("codex.js")
                            || content.starts_with("#!/usr/bin/env node")
                            || content.contains("import");
                        if is_wrapper {
                            if wrapper_candidate.is_none() {
                                wrapper_candidate = Some(candidate.clone());
                                log::debug!(
                                    "Found wrapper script candidate at {} (will use only if no native binary is found)",
                                    candidate.display()
                                );
                            }
                            continue;
                        }
                    }
                    log::debug!("Found codex in PATH at {}", candidate.display());
                    return Some(candidate);
                }
            }
        }
        if let Some(wrapper) = wrapper_candidate {
            log::debug!(
                "Using wrapper codex from PATH at {} as fallback",
                wrapper.display()
            );
            return Some(wrapper);
        }
    }

    // On Windows, also check WSL distributions
    #[cfg(windows)]
    {
        if let Ok(distros) = wsl::list_distros() {
            for distro in distros {
                // Try to find codex in WSL PATH
                if let Ok(output) = wsl::exec_in_wsl(&distro.name, "which codex") {
                    if !output.is_empty() {
                        log::debug!("Found codex in WSL {} at {}", distro.name, output);
                        // Return a special marker that indicates WSL path
                        // Format: wsl://<distro>/<wsl_path>
                        return Some(PathBuf::from(format!(
                            "wsl://{}/{}",
                            distro.name,
                            output.trim()
                        )));
                    }
                }

                // Also check common WSL installation locations
                let wsl_paths = [
                    "/usr/local/bin/codex",
                    "/usr/bin/codex",
                    "$HOME/.cargo/bin/codex",
                    "$HOME/.bun/bin/codex",
                ];

                for wsl_path in &wsl_paths {
                    let check_cmd = format!("test -f {} && echo {}", wsl_path, wsl_path);
                    if let Ok(output) = wsl::exec_in_wsl(&distro.name, &check_cmd) {
                        if !output.is_empty() {
                            log::debug!("Found codex in WSL {} at {}", distro.name, output.trim());
                            return Some(PathBuf::from(format!(
                                "wsl://{}/{}",
                                distro.name,
                                output.trim()
                            )));
                        }
                    }
                }
            }
        }
    }

    log::warn!("No codex binary found in common locations or PATH");
    None
}

/// Get all potential codex root directories including WSL on Windows
pub fn get_codex_root_candidates() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // Add standard locations
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        roots.push(PathBuf::from(&home).join(".codex"));
        roots.push(PathBuf::from(&home).join("code"));
        roots.push(PathBuf::from(&home));
    }

    // On Windows, also add WSL locations
    #[cfg(windows)]
    {
        roots.extend(wsl::get_wsl_codex_roots());
    }

    roots
}

/// Get all potential session storage directories including WSL
pub fn get_session_root_candidates() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // Add standard .codex/sessions location
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        roots.push(PathBuf::from(&home).join(".codex").join("sessions"));
    }

    // On Windows, also check WSL distributions
    #[cfg(windows)]
    {
        if let Ok(distros) = wsl::list_distros() {
            for distro in distros {
                if let Ok(home) = wsl::get_distro_home(&distro.name) {
                    let sessions_path = format!("{}/.codex/sessions", home);
                    roots.push(PathBuf::from(wsl::wsl_to_unc(&sessions_path, &distro.name)));
                }
            }
        }
    }

    roots
}
