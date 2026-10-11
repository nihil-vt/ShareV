//! Upload / share destinations.
//!
//! Three interchangeable uploaders are supported:
//!
//! * `imgur`   — anonymous Imgur upload via a Client-ID.
//! * `http`    — a generic `multipart/form-data` POST.
//! * `command` — run a user-defined shell command and read the URL from stdout.
//!
//! Every uploader takes a local file path and returns the resulting URL.

use std::path::Path;

use reqwest::multipart::{Form, Part};

use crate::config::{Config, UploaderKind};
use crate::error::{Error, Result};

/// Upload `path` using the destination configured in `config`.
pub async fn upload(config: &Config, path: &Path) -> Result<String> {
    match config.upload.uploader {
        UploaderKind::None => Err(Error::msg(
            "no upload destination is configured (Settings → Uploads)",
        )),
        UploaderKind::Imgur => upload_imgur(config, path).await,
        UploaderKind::Http => upload_http(config, path).await,
        UploaderKind::Command => upload_command(config, path).await,
    }
}

/// Read the file and build a multipart part carrying its name and mime type.
fn file_part(path: &Path) -> Result<Part> {
    let bytes = std::fs::read(path)?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".into());
    Part::bytes(bytes)
        .file_name(file_name)
        .mime_str(mime_for(path))
        .map_err(|e| Error::Http(e.to_string()))
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

async fn upload_imgur(config: &Config, path: &Path) -> Result<String> {
    let client_id = config.upload.imgur_client_id.trim();
    if client_id.is_empty() {
        return Err(Error::msg(
            "Imgur needs a Client-ID — add one in Settings → Uploads",
        ));
    }

    let form = Form::new().part("image", file_part(path)?);
    let response = reqwest::Client::new()
        .post("https://api.imgur.com/3/image")
        .header("Authorization", format!("Client-ID {client_id}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| Error::msg(format!("Imgur reply: {e}")))?;

    if !status.is_success() {
        let message = value
            .pointer("/data/error")
            .and_then(|v| v.as_str())
            .unwrap_or(&body);
        return Err(Error::msg(format!("Imgur upload failed ({status}): {message}")));
    }

    value
        .pointer("/data/link")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| Error::msg("Imgur reply did not include a link"))
}

async fn upload_http(config: &Config, path: &Path) -> Result<String> {
    let url = config.upload.http_url.trim();
    if url.is_empty() {
        return Err(Error::msg("no HTTP upload URL is configured"));
    }

    let field = match config.upload.http_field.trim() {
        "" => "file",
        name => name,
    };

    let form = Form::new().part(field.to_string(), file_part(path)?);
    let mut request = reqwest::Client::new().post(url).multipart(form);

    for (name, value) in parse_headers(&config.upload.http_headers) {
        request = request.header(name, value);
    }

    let response = request
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;

    if !status.is_success() {
        return Err(Error::msg(format!("upload failed ({status}): {}", body.trim())));
    }

    extract_url(&body, &config.upload.http_response_path)
}

/// Pull the URL out of an upload response.
///
/// When `path` is empty the trimmed body is used as-is. Otherwise `path` is a
/// dotted JSON path such as `data.link` or `files.0.url`.
fn extract_url(body: &str, path: &str) -> Result<String> {
    let body = body.trim();
    if path.trim().is_empty() {
        if body.is_empty() {
            return Err(Error::msg("upload response was empty"));
        }
        return Ok(body.to_string());
    }

    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| Error::msg(format!("upload response is not JSON: {e}")))?;

    let mut current = &value;
    for segment in path.trim().split('.') {
        current = if let Ok(index) = segment.parse::<usize>() {
            current.get(index)
        } else {
            current.get(segment)
        }
        .ok_or_else(|| Error::msg(format!("upload response has no field '{segment}'")))?;
    }

    current
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| Error::msg("upload response field is not a string"))
}

/// Parse "Name: Value" lines into header pairs, ignoring blanks and comments.
fn parse_headers(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (name, value) = line.split_once(':')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some((name.to_string(), value.trim().to_string()))
        })
        .collect()
}

/// Quote a string for safe interpolation into a POSIX `sh -c` command.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn upload_command(config: &Config, path: &Path) -> Result<String> {
    let template = config.upload.command.trim();
    if template.is_empty() {
        return Err(Error::msg("no upload command is configured"));
    }

    let file = path.to_string_lossy().into_owned();
    let command = if template.contains("{file}") {
        template.replace("{file}", &shell_quote(&file))
    } else {
        format!("{template} {}", shell_quote(&file))
    };

    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .env("SHAREV_FILE", &file)
        .output()
        .await
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::msg(format!(
            "upload command failed: {}",
            stderr.trim()
        )));
    }

    // Many CLIs print progress and then the URL, so use the last non-empty line.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let url = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();

    if url.is_empty() {
        return Err(Error::msg("upload command printed no URL on stdout"));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_lines() {
        let raw = "# comment\nAuthorization: Bearer abc\n\nX-Custom:value:with:colons\n";
        let headers = parse_headers(raw);
        assert_eq!(
            headers,
            vec![
                ("Authorization".to_string(), "Bearer abc".to_string()),
                ("X-Custom".to_string(), "value:with:colons".to_string()),
            ]
        );
    }

    #[test]
    fn extracts_url_from_raw_body() {
        assert_eq!(
            extract_url("https://x/y.png\n", "").unwrap(),
            "https://x/y.png"
        );
        assert!(extract_url("   ", "").is_err());
    }

    #[test]
    fn extracts_url_from_json_path() {
        let body = r#"{"data":{"link":"https://i.imgur.com/abc.png"}}"#;
        assert_eq!(
            extract_url(body, "data.link").unwrap(),
            "https://i.imgur.com/abc.png"
        );

        let array = r#"{"files":[{"url":"https://a/1.png"}]}"#;
        assert_eq!(
            extract_url(array, "files.0.url").unwrap(),
            "https://a/1.png"
        );

        assert!(extract_url(body, "data.missing").is_err());
        assert!(extract_url("not json", "data.link").is_err());
    }

    #[test]
    fn quotes_shell_arguments() {
        assert_eq!(shell_quote("/tmp/a b.png"), "'/tmp/a b.png'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn detects_mime_types() {
        assert_eq!(mime_for(Path::new("a.jpg")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.webp")), "image/webp");
        assert_eq!(mime_for(Path::new("a.gif")), "image/gif");
        assert_eq!(mime_for(Path::new("a.png")), "image/png");
        assert_eq!(mime_for(Path::new("a")), "image/png");
    }
}
