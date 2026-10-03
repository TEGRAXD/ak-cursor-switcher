use std::fs;
use std::path::PathBuf;

// Application configuration persisted to `settings.ini` next to the executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub target_process: String,
    pub scheme: String,
    pub size: u32,
    pub auto_switch: bool,
    pub game_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            target_process: String::from("Arknights.exe"),
            scheme: String::new(),
            size: 1,
            auto_switch: true,
            game_path: String::new(),
        }
    }
}

impl AppConfig {
    // Resolves path to `settings.ini` next to current executable, falling back to working directory.
    fn config_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("settings.ini")))
            .unwrap_or_else(|| PathBuf::from("settings.ini"))
    }

    // Loads configuration from `settings.ini`. Returns default configuration if file is missing or invalid.
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            Self::parse_from_str(&content)
        } else {
            Self::default()
        }
    }

    // Parses configuration key-values from INI formatted text.
    pub fn parse_from_str(content: &str) -> Self {
        let mut config = Self::default();
        for line in content.lines() {
            let trimmed = line.trim();
            // Skip empty lines and comment lines starting with # or ;
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if let Some((key, val)) = trimmed.split_once('=') {
                match key.trim() {
                    "target_process" => config.target_process = val.trim().to_string(),
                    "scheme" => config.scheme = val.trim().to_string(),
                    "size" => {
                        if let Ok(s) = val.trim().parse::<u32>() {
                            config.size = s.clamp(1, 6);
                        }
                    }
                    "auto_switch" => {
                        if let Ok(b) = val.trim().parse::<bool>() {
                            config.auto_switch = b;
                        }
                    }
                    "game_path" => config.game_path = val.trim().to_string(),
                    _ => {}
                }
            }
        }
        config
    }

    // Saves current configuration to `settings.ini`.
    pub fn save(&self) -> bool {
        let path = Self::config_path();
        let content = format!(
            "# Cursor Switcher Preferences\ntarget_process={}\nscheme={}\nsize={}\nauto_switch={}\ngame_path={}\n",
            self.target_process, self.scheme, self.size, self.auto_switch, self.game_path
        );
        fs::write(path, content).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.target_process, "Arknights.exe");
        assert_eq!(cfg.size, 1);
        assert!(cfg.auto_switch);
    }

    #[test]
    fn test_parse_custom_config() {
        let ini = "target_process=Game.exe\nscheme=Windows Aero L\nsize=2\nauto_switch=false\n";
        let cfg = AppConfig::parse_from_str(ini);
        assert_eq!(cfg.target_process, "Game.exe");
        assert_eq!(cfg.scheme, "Windows Aero L");
        assert_eq!(cfg.size, 2);
        assert!(!cfg.auto_switch);
    }
}
