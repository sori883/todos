use crate::config::settings::Settings;
use crate::error::AppError;

/// Remove control characters from text, optionally keeping newlines.
fn strip_control_chars(s: &str, keep_newlines: bool) -> String {
    s.chars()
        .filter(|c| {
            if keep_newlines && *c == '\n' {
                return true;
            }
            !c.is_control()
        })
        .collect()
}

/// Sanitize a task title: trim, strip control chars, reject empty/too long.
pub fn sanitize_title(title: &str, settings: &Settings) -> Result<String, AppError> {
    let max_len = settings.max_title_length;
    let sanitized = strip_control_chars(title.trim(), false);

    if sanitized.is_empty() {
        return Err(AppError::InvalidInput("Title cannot be empty".to_string()));
    }

    if sanitized.chars().count() > max_len {
        return Err(AppError::InvalidInput(format!(
            "Title too long: {} chars (max {})",
            sanitized.chars().count(),
            max_len
        )));
    }

    Ok(sanitized)
}

/// Sanitize task content: trim, strip control chars (keep newlines), reject too long.
/// Returns None if content is empty after sanitization.
pub fn sanitize_content(content: &str, settings: &Settings) -> Result<Option<String>, AppError> {
    let max_len = settings.max_content_length;
    let sanitized = strip_control_chars(content.trim(), true);

    if sanitized.is_empty() {
        return Ok(None);
    }

    if sanitized.chars().count() > max_len {
        return Err(AppError::InvalidInput(format!(
            "Content too long: {} chars (max {})",
            sanitized.chars().count(),
            max_len
        )));
    }

    Ok(Some(sanitized))
}

/// Sanitize a project name: trim, strip control chars, reject too long.
/// Returns None if empty after sanitization.
pub fn sanitize_project(project: &str, settings: &Settings) -> Result<Option<String>, AppError> {
    let max_len = settings.max_project_length;
    let sanitized = strip_control_chars(project.trim(), false);

    if sanitized.is_empty() {
        return Ok(None);
    }

    if sanitized.chars().count() > max_len {
        return Err(AppError::InvalidInput(format!(
            "Project name too long: {} chars (max {})",
            sanitized.chars().count(),
            max_len
        )));
    }

    Ok(Some(sanitized))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn title_trims_whitespace() {
        let s = default_settings();
        assert_eq!(sanitize_title("  hello  ", &s).unwrap(), "hello");
    }

    #[test]
    fn title_strips_control_chars() {
        let s = default_settings();
        assert_eq!(sanitize_title("he\x00ll\x07o", &s).unwrap(), "hello");
    }

    #[test]
    fn title_strips_newlines() {
        let s = default_settings();
        assert_eq!(sanitize_title("line1\nline2", &s).unwrap(), "line1line2");
    }

    #[test]
    fn title_rejects_empty() {
        let s = default_settings();
        assert!(sanitize_title("", &s).is_err());
        assert!(sanitize_title("   ", &s).is_err());
        assert!(sanitize_title("\n\t", &s).is_err());
    }

    #[test]
    fn title_rejects_too_long() {
        let s = default_settings();
        let long = "a".repeat(201);
        assert!(sanitize_title(&long, &s).is_err());
        let ok = "a".repeat(200);
        assert!(sanitize_title(&ok, &s).is_ok());
    }

    #[test]
    fn title_respects_custom_limit() {
        let mut s = default_settings();
        s.max_title_length = 5;
        assert!(sanitize_title("abcde", &s).is_ok());
        assert!(sanitize_title("abcdef", &s).is_err());
    }

    #[test]
    fn content_trims_and_keeps_newlines() {
        let s = default_settings();
        let result = sanitize_content("  line1\nline2  ", &s).unwrap();
        assert_eq!(result, Some("line1\nline2".to_string()));
    }

    #[test]
    fn content_strips_control_chars_except_newlines() {
        let s = default_settings();
        let result = sanitize_content("a\x00b\nc\x07d", &s).unwrap();
        assert_eq!(result, Some("ab\ncd".to_string()));
    }

    #[test]
    fn content_returns_none_when_empty() {
        let s = default_settings();
        assert_eq!(sanitize_content("", &s).unwrap(), None);
        assert_eq!(sanitize_content("   ", &s).unwrap(), None);
    }

    #[test]
    fn content_rejects_too_long() {
        let s = default_settings();
        let long = "a".repeat(10_001);
        assert!(sanitize_content(&long, &s).is_err());
        let ok = "a".repeat(10_000);
        assert!(sanitize_content(&ok, &s).is_ok());
    }

    #[test]
    fn content_respects_custom_limit() {
        let mut s = default_settings();
        s.max_content_length = 10;
        assert!(sanitize_content("1234567890", &s).is_ok());
        assert!(sanitize_content("12345678901", &s).is_err());
    }

    #[test]
    fn project_trims_and_strips() {
        let s = default_settings();
        assert_eq!(
            sanitize_project("  my-proj\x00  ", &s).unwrap(),
            Some("my-proj".to_string())
        );
    }

    #[test]
    fn project_returns_none_when_empty() {
        let s = default_settings();
        assert_eq!(sanitize_project("", &s).unwrap(), None);
    }

    #[test]
    fn project_rejects_too_long() {
        let s = default_settings();
        let long = "a".repeat(101);
        assert!(sanitize_project(&long, &s).is_err());
    }

    #[test]
    fn title_handles_unicode() {
        let s = default_settings();
        assert_eq!(
            sanitize_title("日本語タイトル", &s).unwrap(),
            "日本語タイトル"
        );
        let jp = "あ".repeat(200);
        assert!(sanitize_title(&jp, &s).is_ok());
        let jp_long = "あ".repeat(201);
        assert!(sanitize_title(&jp_long, &s).is_err());
    }

    #[test]
    fn content_handles_unicode_multiline() {
        let s = default_settings();
        let input = "1行目\n2行目\n3行目";
        let result = sanitize_content(input, &s).unwrap();
        assert_eq!(result, Some("1行目\n2行目\n3行目".to_string()));
    }
}
