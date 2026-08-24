use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::system::processes::SortKey;

/// Column identifiers usable in the `columns` config entry.
pub const KNOWN_COLUMNS: [&str; 9] = [
    "pid",
    "name",
    "cpu",
    "mem",
    "mem_percent",
    "user",
    "state",
    "virt",
    "time",
];

/// User configuration loaded from a TOML file.
///
/// Every field has a default, so partial or even empty config files are
/// valid. Unknown keys are ignored to keep forward compatibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Refresh interval in milliseconds.
    pub refresh_ms: u64,
    /// Initial process-list sort key.
    pub sort_by: SortKey,
    /// Whether the initial sort is descending.
    pub sort_descending: bool,
    /// Maximum number of samples kept for graphs/history (bounded memory).
    pub history_size: usize,
    /// Color theme name (`default`, `ocean`, `mono`).
    pub theme: String,
    /// Visible process-table columns, in order.
    pub columns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_ms: 1000,
            sort_by: SortKey::Cpu,
            sort_descending: true,
            history_size: 120,
            theme: "default".to_string(),
            columns: vec![
                "pid".into(),
                "name".into(),
                "cpu".into(),
                "mem".into(),
                "user".into(),
                "state".into(),
            ],
        }
    }
}

impl Config {
    /// Clamp values into sane ranges so a hand-edited config cannot cause
    /// pathological behaviour (e.g. a zero refresh interval spinning the CPU).
    pub fn sanitized(mut self) -> Self {
        self.refresh_ms = self.refresh_ms.clamp(100, 60_000);
        self.history_size = self.history_size.clamp(10, 3_600);
        if !matches!(self.theme.as_str(), "default" | "ocean" | "mono") {
            self.theme = "default".into();
        }
        let valid: Vec<String> = self
            .columns
            .iter()
            .filter(|c| KNOWN_COLUMNS.contains(&c.as_str()))
            .cloned()
            .collect();
        // Drop duplicates while preserving order.
        self.columns.clear();
        for c in valid {
            if !self.columns.contains(&c) {
                self.columns.push(c);
            }
        }
        if self.columns.is_empty() {
            self.columns = Config::default().columns;
        }
        self
    }

    /// Parse a config from TOML text. Unknown fields are ignored; missing
    /// fields fall back to defaults via `#[serde(default)]`.
    pub fn parse_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Default config location: `$XDG_CONFIG_HOME/rtop/config.toml`
    /// (falling back to `$HOME/.config`).
    pub fn default_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
            })?;
        Some(base.join("rtop").join("config.toml"))
    }

    /// Load from `path`. A missing file yields the default config; a broken
    /// file is reported as an error so the caller can warn without crashing.
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::parse_toml(&text).map_err(|e| format!("{path:?}: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(format!("{path:?}: {e}")),
        }
    }

    /// Load from the default location if present.
    pub fn load_default() -> (Self, Option<String>) {
        match Config::default_path() {
            Some(path) => match Config::load(&path) {
                Ok(cfg) => (cfg.sanitized(), None),
                Err(e) => (
                    Config::default().sanitized(),
                    Some(format!("Ignoring invalid config: {e}")),
                ),
            },
            None => (Config::default().sanitized(), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.refresh_ms, 1000);
        assert_eq!(cfg.sort_by, SortKey::Cpu);
        assert!(cfg.sort_descending);
        assert!(!cfg.columns.is_empty());
    }

    #[test]
    fn parses_full_document() {
        let cfg = Config::parse_toml(
            r#"
refresh_ms = 500
sort_by = "memory"
sort_descending = false
history_size = 300
theme = "ocean"
columns = ["name", "pid", "cpu"]
"#,
        )
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
        assert_eq!(cfg.refresh_ms, 500);
        assert_eq!(cfg.sort_by, SortKey::Memory);
        assert!(!cfg.sort_descending);
        assert_eq!(cfg.history_size, 300);
        assert_eq!(cfg.theme, "ocean");
        assert_eq!(cfg.columns, vec!["name", "pid", "cpu"]);
    }

    #[test]
    fn empty_document_yields_defaults() {
        let cfg = Config::parse_toml("").unwrap_or_else(|e| panic!("parse failed: {e}"));
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn unknown_keys_and_columns_are_ignored() {
        let cfg = Config::parse_toml("bogus_key = true\ncolumns = [\"pid\", \"nope\"]")
            .unwrap_or_else(|e| panic!("parse failed: {e}"))
            .sanitized();
        assert_eq!(cfg.columns, vec!["pid"]);
    }

    #[test]
    fn sanitize_clamps_ranges() {
        let cfg = Config::parse_toml("refresh_ms = 1\nhistory_size = 999999\ntheme = \"neon\"")
            .unwrap_or_else(|e| panic!("parse failed: {e}"))
            .sanitized();
        assert_eq!(cfg.refresh_ms, 100);
        assert_eq!(cfg.history_size, 3600);
        assert_eq!(cfg.theme, "default");
    }

    #[test]
    fn sanitize_dedups_columns_and_fills_defaults_when_empty() {
        let cfg = Config::parse_toml("columns = [\"cpu\", \"cpu\"]")
            .unwrap_or_else(|e| panic!("parse failed: {e}"))
            .sanitized();
        assert_eq!(cfg.columns, vec!["cpu"]);

        let cfg = Config::parse_toml("columns = [\"zzz\"]")
            .unwrap_or_else(|e| panic!("parse failed: {e}"))
            .sanitized();
        assert_eq!(cfg.columns, Config::default().columns);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let cfg = Config::load(Path::new("/nonexistent/rtop/config.toml"));
        assert!(matches!(cfg, Ok(c) if c == Config::default()));
    }

    #[test]
    fn load_broken_file_is_error_not_panic() {
        let dir = std::env::temp_dir().join("rtop-test-broken.toml");
        std::fs::write(&dir, "refresh_ms = \"not a number\"").unwrap_or_else(|e| {
            panic!("write failed: {e}")
        });
        assert!(Config::load(&dir).is_err());
        std::fs::remove_file(&dir).ok();
    }
}
