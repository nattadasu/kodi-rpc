pub mod image_utils;
pub mod imgur;
pub mod litterbox;

use crate::KodiResult;

/// Collapse whitespace and truncate a response body for error messages
/// (Catbox returns full HTML error pages on 502s — logging all of it
/// spams the journal).
pub(crate) fn snippet(s: &str) -> String {
    const MAX: usize = 200;
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > MAX {
        format!("{}…", flat.chars().take(MAX).collect::<String>())
    } else {
        flat
    }
}

/// Read an upload response body, turning HTTP errors into actionable errors
/// (`{service} HTTP 502: ...`) instead of confusing downstream parse
/// failures (`relative URL without a base`, `error decoding response
/// body`). Success bodies are trimmed for the caller to validate.
pub(crate) fn upload_body(
    service: &'static str,
    resp: reqwest::blocking::Response,
) -> KodiResult<String> {
    let status = resp.status();
    let body = resp.text()?.trim().to_string();
    if !status.is_success() {
        return Err(format!("{service} HTTP {status}: {}", snippet(&body)).into());
    }
    Ok(body)
}
