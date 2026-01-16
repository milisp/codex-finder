use std::path::PathBuf;

fn get_platform_binary_name() -> &'static str {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "aarch64") => "code-aarch64-apple-darwin",
        ("macos", "x86_64") => "coder-x86_64-apple-darwin",
        ("linux", "x86_64") => "coder-x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "coder-aarch64-unknown-linux-musl",
        ("windows", "x86_64") => "coder-x86_64-pc-windows-msvc.exe",
        _ => "coder", // fallback
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
    if cfg!(windows) { "coder.exe" } else { "coder" }
}

pub fn discover_coder_command() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_default()
    } else {
        std::env::var("HOME").unwrap_or_default()
    };
    let binary_name = get_platform_binary_name();

    // 0) Optional override via environment variable
    if let Ok(explicit) = std::env::var("CODER_PATH") {
        let p = PathBuf::from(&explicit);
        if p.exists() {
            log::debug!("Using CODER_PATH override at {}", p.display());
            return Some(p);
        } else {
            log::warn!("CODER_PATH provided but not found: {}", explicit);
        }
    }

    // First priority: Check actual binary locations in node_modules
    let binary_locations = [
        // Bun global installation
        PathBuf::from(&home)
            .join(".bun/install/global/node_modules/@just-every/code/bin")
            .join(binary_name),
        // NPM rootless (user) global installation
        PathBuf::from(&home)
            .join(".local/share/npm/lib/node_modules/@just-every/code/bin")
            .join(binary_name),
        // NPM global installations
        PathBuf::from("/usr/local/lib/node_modules/@just-every/code/bin").join(binary_name),
        PathBuf::from("/opt/homebrew/lib/node_modules/@just-every/code/bin").join(binary_name),
    ];

    for path_buf in &binary_locations {
        if path_buf.exists() {
            log::debug!("Found coder binary at {}", path_buf.display());
            return Some(path_buf.clone());
        }
    }

    if let Some(vendor_dir) = get_vendor_platform_dir() {
        let vendor_binary = get_vendor_binary_name();
        let vendor_locations = [
            PathBuf::from(&home)
                .join(".bun/install/global/node_modules/@just-every/code/vendor")
                .join(vendor_dir)
                .join("coder")
                .join(vendor_binary),
            PathBuf::from(&home)
                .join(".local/share/npm/lib/node_modules/@just-every/code/vendor")
                .join(vendor_dir)
                .join("coder")
                .join(vendor_binary),
            PathBuf::from("/usr/local/lib/node_modules/@just-every/code/vendor")
                .join(vendor_dir)
                .join("coder")
                .join(vendor_binary),
            PathBuf::from("/opt/homebrew/lib/node_modules/@just-every/code/vendor")
                .join(vendor_dir)
                .join("coder")
                .join(vendor_binary),
        ];

        for path_buf in &vendor_locations {
            if path_buf.exists() {
                log::debug!("Found coder vendor binary at {}", path_buf.display());
                return Some(path_buf.clone());
            }
        }
    }

    // Windows npm global installation paths
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let npm_paths = [
                PathBuf::from(&appdata).join("npm").join("coder.cmd"),
                PathBuf::from(&appdata).join("npm").join("coder.ps1"),
                PathBuf::from(&appdata).join("npm").join("coder"),
            ];
            for path_buf in &npm_paths {
                if path_buf.exists() {
                    log::debug!("Found npm coder at {}", path_buf.display());
                    return Some(path_buf.clone());
                }
            }
        }
    }

    // Second priority: Check if there are native rust/cargo installations
    let native_paths = [
        PathBuf::from(&home).join(".cargo/bin/coder"),
        PathBuf::from(&home).join(".cargo/bin/coder.exe"),
        PathBuf::from("/usr/local/bin/coder"),
        PathBuf::from("/opt/homebrew/bin/coder"),
    ];

    for path_buf in &native_paths {
        if path_buf.exists() {
            // Check if it's a real binary (not a js wrapper)
            if let Ok(content) = std::fs::read_to_string(path_buf) {
                if content.contains("coder.js") || content.starts_with("#!/usr/bin/env node") {
                    // This is a wrapper script, skip it
                    log::debug!("Skipping wrapper script at {}", path_buf.display());
                    continue;
                }
            }
            log::debug!("Found native coder binary at {}", path_buf.display());
            return Some(path_buf.clone());
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        let mut wrapper_candidate: Option<PathBuf> = None;
        let candidate_names: &[&str] = if cfg!(windows) {
            &["coder.exe", "coder.cmd", "coder.ps1", "coder"]
        } else {
            &["coder"]
        };
        for dir in path_env.split(separator) {
            if dir.is_empty() {
                continue;
            }
            for name in candidate_names {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.exists() {
                    if let Ok(content) = std::fs::read_to_string(&candidate) {
                        let is_wrapper = content.contains("coder.js")
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
                    log::debug!("Found coder in PATH at {}", candidate.display());
                    return Some(candidate);
                }
            }
        }
        if let Some(wrapper) = wrapper_candidate {
            log::debug!(
                "Using wrapper coder from PATH at {} as fallback",
                wrapper.display()
            );
            return Some(wrapper);
        }
    }

    log::warn!("No coder binary found in common locations or PATH");
    None
}
