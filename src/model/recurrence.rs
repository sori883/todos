use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DayOfWeek {
    /// Parse from CLI short form (e.g., "mon", "tue")
    pub fn from_short(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mon" => Some(Self::Monday),
            "tue" => Some(Self::Tuesday),
            "wed" => Some(Self::Wednesday),
            "thu" => Some(Self::Thursday),
            "fri" => Some(Self::Friday),
            "sat" => Some(Self::Saturday),
            "sun" => Some(Self::Sunday),
            _ => None,
        }
    }

    /// Convert to chrono::Weekday
    pub fn to_chrono_weekday(self) -> chrono::Weekday {
        match self {
            Self::Monday => chrono::Weekday::Mon,
            Self::Tuesday => chrono::Weekday::Tue,
            Self::Wednesday => chrono::Weekday::Wed,
            Self::Thursday => chrono::Weekday::Thu,
            Self::Friday => chrono::Weekday::Fri,
            Self::Saturday => chrono::Weekday::Sat,
            Self::Sunday => chrono::Weekday::Sun,
        }
    }

    /// Convert from chrono::Weekday
    pub fn from_chrono_weekday(wd: chrono::Weekday) -> Self {
        match wd {
            chrono::Weekday::Mon => Self::Monday,
            chrono::Weekday::Tue => Self::Tuesday,
            chrono::Weekday::Wed => Self::Wednesday,
            chrono::Weekday::Thu => Self::Thursday,
            chrono::Weekday::Fri => Self::Friday,
            chrono::Weekday::Sat => Self::Saturday,
            chrono::Weekday::Sun => Self::Sunday,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    #[default]
    Never,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    DaysOfWeek(Vec<DayOfWeek>),
}

impl Recurrence {
    /// Parse from CLI string (e.g., "never", "daily", "mon,wed,fri")
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "never" => Ok(Self::Never),
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "yearly" => Ok(Self::Yearly),
            other => {
                // Try parsing as comma-separated days
                let days: Result<Vec<DayOfWeek>, String> = other
                    .split(',')
                    .map(|d| {
                        DayOfWeek::from_short(d.trim())
                            .ok_or_else(|| format!("Invalid day: '{}'", d.trim()))
                    })
                    .collect();
                match days {
                    Ok(d) if d.is_empty() => Err(format!("Invalid recurrence: '{s}'")),
                    Ok(d) => Ok(Self::DaysOfWeek(d)),
                    Err(e) => Err(e),
                }
            }
        }
    }
}
