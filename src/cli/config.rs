use std::path::Path;

use crate::cli::output::{print_response, CliResponse};
use crate::config::settings::Settings;
use crate::error::AppError;

pub struct ConfigParams {
    pub show: bool,
    pub mode: Option<String>,
    pub icons: Option<String>,
    pub reset: bool,
    pub yes: bool,
}

pub fn run(
    data_dir: &Path,
    params: ConfigParams,
    format: &str,
) -> Result<(), AppError> {
    if params.reset {
        let deleted = Settings::reset(data_dir)?;
        if deleted {
            let response = CliResponse::<()>::message_only(
                "Local settings reset".to_string(),
            );
            print_response(&response, format);
        } else {
            let response = CliResponse::<()>::message_only(
                "No local settings to reset".to_string(),
            );
            print_response(&response, format);
        }
        return Ok(());
    }

    // Apply mode/icons changes if specified
    let has_changes = params.mode.is_some() || params.icons.is_some();
    if has_changes {
        let mut settings = Settings::load(data_dir)?;

        if let Some(ref mode) = params.mode {
            match mode.as_str() {
                "vi" | "default" => {
                    settings.keybindings.mode = mode.clone();
                }
                other => {
                    return Err(AppError::InvalidInput(format!(
                        "Invalid keybinding mode: '{other}' (valid: vi, default)"
                    )));
                }
            }
        }

        if let Some(ref icons) = params.icons {
            match icons.as_str() {
                "nerd" | "chars" => {
                    settings.icons.style = icons.clone();
                }
                other => {
                    return Err(AppError::InvalidInput(format!(
                        "Invalid icon style: '{other}' (valid: nerd, chars)"
                    )));
                }
            }
        }

        settings.save(data_dir)?;

        let response = CliResponse::<()>::message_only("Settings updated".to_string());
        print_response(&response, format);
        return Ok(());
    }

    // Default: show settings (same as --show)
    if params.show || !has_changes {
        let settings = Settings::load(data_dir)?;
        let response = CliResponse::success(settings);
        print_response(&response, format);
    }

    Ok(())
}
