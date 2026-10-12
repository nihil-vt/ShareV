//! OCR (Optical Character Recognition) using Tesseract.
//!
//! Runs the `tesseract` CLI on an image file and returns the recognized text.
//! The Tesseract binary must be installed on the host system.

use std::process::Stdio;

use tokio::process::Command;

use crate::error::{Error, Result};

/// Run OCR on `path` using Tesseract with the given language.
///
/// Returns the recognized text (trimmed), or an error if Tesseract is not
/// installed or fails.
pub async fn recognize(path: &str, language: &str) -> Result<String> {
    // Check if tesseract is available first.
    let check = Command::new("which")
        .arg("tesseract")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(Error::Io)?;

    if !check.success() {
        return Err(Error::msg(
            "tesseract is not installed — install it to use OCR",
        ));
    }

    let lang = if language.trim().is_empty() {
        "eng"
    } else {
        language.trim()
    };

    // Run tesseract: tesseract <path> stdout -l <lang>
    let output = Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg(lang)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::msg(format!("tesseract failed: {}", stderr.trim())));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.trim().to_string())
}

/// Check if Tesseract is available and list installed languages.
pub async fn check_available() -> (bool, Vec<String>) {
    let check = Command::new("which")
        .arg("tesseract")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    let available = check.map(|s| s.success()).unwrap_or(false);
    if !available {
        return (false, Vec::new());
    }

    // List installed languages.
    let output = Command::new("tesseract")
        .arg("--list-langs")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    let langs = match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .skip(1) // Skip header line "List of available languages"
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => Vec::new(),
    };

    (true, langs)
}
