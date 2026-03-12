use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindSettings {
    pub mode: String,
}

impl Default for KeybindSettings {
    fn default() -> Self {
        Self {
            mode: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconSettings {
    pub style: String,
}

impl Default for IconSettings {
    fn default() -> Self {
        Self {
            style: "chars".to_string(),
        }
    }
}

/// Partial settings for deserialization from JSON files.
/// All fields are optional so missing fields use defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PartialSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keybindings: Option<PartialKeybindSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<PartialIconSettings>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_projects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_title_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_project_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PartialKeybindSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PartialIconSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub locale: String,
    pub keybindings: KeybindSettings,
    pub icons: IconSettings,
    pub extra_labels: Vec<String>,
    pub extra_projects: Vec<String>,
    pub builtin_labels: Vec<String>,
    pub max_title_length: usize,
    pub max_content_length: usize,
    pub max_project_length: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "ja".to_string(),
            keybindings: KeybindSettings::default(),
            icons: IconSettings::default(),
            extra_labels: Vec::new(),
            extra_projects: Vec::new(),
            builtin_labels: Self::default_builtin_labels(),
            max_title_length: 200,
            max_content_length: 10_000,
            max_project_length: 100,
        }
    }
}

impl Settings {
    fn default_builtin_labels() -> Vec<String> {
        vec![
            "bug".to_string(),
            "feature".to_string(),
            "improvement".to_string(),
            "documentation".to_string(),
            "refactor".to_string(),
            "chore".to_string(),
        ]
    }

    /// Return all allowed labels (builtin + extra).
    pub fn all_labels(&self) -> Vec<String> {
        let mut labels = self.builtin_labels.clone();
        for label in &self.extra_labels {
            if !labels.contains(label) {
                labels.push(label.clone());
            }
        }
        labels
    }

    /// Load settings from the data directory.
    /// Reads `.todos/settings.json` if it exists, merges with defaults.
    pub fn load(data_dir: &Path) -> Result<Self, AppError> {
        let settings_path = data_dir.join("settings.json");
        let mut settings = Self::default();

        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            let partial: PartialSettings = serde_json::from_str(&content)
                .map_err(|e| AppError::Config(format!("Failed to parse settings.json: {e}")))?;

            settings.apply_partial(&partial);
        }

        Ok(settings)
    }

    /// Save current settings to the data directory.
    /// Only saves the user-configurable fields (not builtin_labels).
    pub fn save(&self, data_dir: &Path) -> Result<(), AppError> {
        let settings_path = data_dir.join("settings.json");

        let partial = PartialSettings {
            locale: Some(self.locale.clone()),
            keybindings: Some(PartialKeybindSettings {
                mode: Some(self.keybindings.mode.clone()),
            }),
            icons: Some(PartialIconSettings {
                style: Some(self.icons.style.clone()),
            }),
            extra_labels: self.extra_labels.clone(),
            extra_projects: self.extra_projects.clone(),
            max_title_length: Some(self.max_title_length),
            max_content_length: Some(self.max_content_length),
            max_project_length: Some(self.max_project_length),
        };

        let content = serde_json::to_string_pretty(&partial)
            .map_err(|e| AppError::Config(format!("Failed to serialize settings: {e}")))?;
        fs::write(&settings_path, content)?;

        Ok(())
    }

    /// Delete the local settings.json file.
    pub fn reset(data_dir: &Path) -> Result<bool, AppError> {
        let settings_path = data_dir.join("settings.json");
        if settings_path.exists() {
            fs::remove_file(&settings_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Apply partial settings on top of current settings.
    fn apply_partial(&mut self, partial: &PartialSettings) {
        if let Some(ref locale) = partial.locale {
            self.locale = locale.clone();
        }
        if let Some(ref kb) = partial.keybindings {
            if let Some(ref mode) = kb.mode {
                self.keybindings.mode = mode.clone();
            }
        }
        if let Some(ref icons) = partial.icons {
            if let Some(ref style) = icons.style {
                self.icons.style = style.clone();
            }
        }
        if !partial.extra_labels.is_empty() {
            for label in &partial.extra_labels {
                if !self.extra_labels.contains(label) {
                    self.extra_labels.push(label.clone());
                }
            }
        }
        if !partial.extra_projects.is_empty() {
            for project in &partial.extra_projects {
                if !self.extra_projects.contains(project) {
                    self.extra_projects.push(project.clone());
                }
            }
        }
        if let Some(v) = partial.max_title_length {
            self.max_title_length = v;
        }
        if let Some(v) = partial.max_content_length {
            self.max_content_length = v;
        }
        if let Some(v) = partial.max_project_length {
            self.max_project_length = v;
        }
    }
}
