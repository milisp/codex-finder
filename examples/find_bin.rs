use codex_finder::{discover_coder_command, discover_codex_command};

fn main() -> Result<(), String> {
    let normalized = "codex";

    let (binary_path, label) = if normalized == "coder" {
        (
            discover_coder_command().ok_or_else(|| {
                "Unable to locate coder binary. Install Coder CLI or set CODER_PATH.".to_string()
            })?,
            "coder",
        )
    } else {
        (
            discover_codex_command().ok_or_else(|| {
                "Unable to locate codex binary. Install Codex CLI or set CODEX_PATH.".to_string()
            })?,
            "codex",
        )
    };

    println!("Found {} binary at {:?}", label, binary_path);

    Ok(())
}