use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CliResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T: Serialize> CliResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: Some(message),
        }
    }
}

impl CliResponse<()> {
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            message: None,
        }
    }

    pub fn message_only(message: String) -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            message: Some(message),
        }
    }
}

/// Print a response in the specified format.
/// For JSON: always output to stdout.
/// For text: output message to stdout, errors to stderr.
pub fn print_response<T: Serialize>(response: &CliResponse<T>, format: &str) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(response).unwrap_or_else(|e| {
                format!("{{\"success\":false,\"error\":\"Serialization failed: {e}\"}}")
            });
            println!("{json}");
        }
        _ => {
            // text mode
            if let Some(ref msg) = response.message {
                println!("{msg}");
            }
            if let Some(ref err) = response.error {
                eprintln!("Error: {err}");
            }
        }
    }
}

/// Print an error response.
/// For JSON: output to stdout as `{"success": false, "error": "..."}`.
/// For text: output to stderr.
pub fn print_error(error: &str, format: &str) {
    match format {
        "json" => {
            let response = CliResponse::<()>::error(error.to_string());
            let json = serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                format!("{{\"success\":false,\"error\":\"Serialization failed: {e}\"}}")
            });
            println!("{json}");
        }
        _ => {
            eprintln!("Error: {error}");
        }
    }
}
