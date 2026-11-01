//! Application settings (spec: doc 08 §1, decision D13).
//!
//! Same keys, types, defaults, and write-moments as the upstream GSettings
//! schema `org.gnome.calculator`, stored in a keyfile at
//! `$XDG_CONFIG_HOME/radix/settings.ini` behind the [`SettingsStore`]
//! trait so a native backend can replace the file backend later.
//! `window-position` is a dead upstream key (never written in v51.1) and
//! is not modeled.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Result-format setting (`number-format`, doc 08 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberFormatSetting {
    Automatic,
    Fixed,
    Scientific,
    Engineering,
}

impl NumberFormatSetting {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Fixed => "fixed",
            Self::Scientific => "scientific",
            Self::Engineering => "engineering",
        }
    }

    pub fn parse_str(s: &str) -> Self {
        match s {
            "fixed" => Self::Fixed,
            "scientific" => Self::Scientific,
            "engineering" => Self::Engineering,
            _ => Self::Automatic,
        }
    }
}

/// Angle-unit setting (`angle-units`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngleSetting {
    Radians,
    Degrees,
    Gradians,
}

impl AngleSetting {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Radians => "radians",
            Self::Degrees => "degrees",
            Self::Gradians => "gradians",
        }
    }

    pub fn parse_str(s: &str) -> Self {
        match s {
            "radians" => Self::Radians,
            "gradians" => Self::Gradians,
            _ => Self::Degrees,
        }
    }
}

/// Currency amount display (`currency-display`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrencyDisplay {
    Code,
    Name,
    Both,
}

impl CurrencyDisplay {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Name => "name",
            Self::Both => "both",
        }
    }

    pub fn parse_str(s: &str) -> Self {
        match s {
            "code" => Self::Code,
            "both" => Self::Both,
            _ => Self::Name,
        }
    }
}

/// Keypad mode (`button-mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonMode {
    Basic,
    Advanced,
    Financial,
    Programming,
    Keyboard,
    Conversion,
}

impl ButtonMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
            Self::Financial => "financial",
            Self::Programming => "programming",
            Self::Keyboard => "keyboard",
            Self::Conversion => "conversion",
        }
    }

    pub fn parse_str(s: &str) -> Self {
        match s {
            "advanced" => Self::Advanced,
            "financial" => Self::Financial,
            "programming" => Self::Programming,
            "keyboard" => Self::Keyboard,
            "conversion" => Self::Conversion,
            _ => Self::Basic,
        }
    }
}

/// One persisted history entry: equation text and the rendered answer
/// (`history-tape`, doc 08 §1; written per window close, doc 06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub equation: String,
    pub answer: String,
}

/// All persisted settings with their schema defaults (doc 08 §1).
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub history_tape: Vec<HistoryEntry>,
    pub accuracy: i32,
    pub word_size: i32,
    pub base: i32,
    pub show_thousands: bool,
    pub show_zeroes: bool,
    pub number_format: NumberFormatSetting,
    pub angle_units: AngleSetting,
    pub refresh_interval: i64,
    pub currency_display: CurrencyDisplay,
    pub enabled_completions: Vec<String>,
    pub favorite_currencies: Vec<String>,
    pub button_mode: ButtonMode,
    pub unit_category: String,
    pub source_units: Vec<String>,
    pub target_units: Vec<String>,
    pub precision: i32,
    pub window_maximized: bool,
    pub window_size: (i32, i32),
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            history_tape: Vec::new(),
            accuracy: 9,
            word_size: 64,
            base: 10,
            show_thousands: false,
            show_zeroes: false,
            number_format: NumberFormatSetting::Automatic,
            angle_units: AngleSetting::Degrees,
            refresh_interval: 604_800,
            currency_display: CurrencyDisplay::Name,
            enabled_completions: vec![
                "builtin".into(),
                "function".into(),
                "currency".into(),
                "favorite".into(),
                "variable".into(),
            ],
            favorite_currencies: Vec::new(),
            button_mode: ButtonMode::Basic,
            unit_category: "angle".into(),
            source_units: vec!["degree".into()],
            target_units: vec!["radian".into()],
            precision: 2000,
            window_maximized: false,
            window_size: (-1, -1),
        }
    }
}

/// Storage backend (D13: swappable; the shipped backend is the keyfile).
pub trait SettingsStore {
    fn load(&self) -> Settings;
    fn save(&self, settings: &Settings);
}

/// Keyfile backend at `$XDG_CONFIG_HOME/radix/settings.ini`.
pub struct KeyfileStore {
    pub path: PathBuf,
}

impl KeyfileStore {
    pub fn default_path() -> PathBuf {
        // $XDG_CONFIG_HOME, falling back to ~/.config (XDG base dir spec).
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty() && PathBuf::from(v).is_absolute())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("radix").join("settings.ini")
    }

    pub fn new() -> Self {
        KeyfileStore {
            path: Self::default_path(),
        }
    }
}

impl Default for KeyfileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsStore for KeyfileStore {
    fn load(&self) -> Settings {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return s;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with('[')
                || line.starts_with('#')
                || line.starts_with(';')
            {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "accuracy" => {
                    if let Ok(v) = value.parse() {
                        s.accuracy = v;
                    }
                }
                "word-size" => {
                    if let Ok(v) = value.parse::<i32>() {
                        s.word_size = v.clamp(8, 64);
                    }
                }
                "base" => {
                    if let Ok(v) = value.parse::<i32>() {
                        s.base = v.clamp(2, 16);
                    }
                }
                "show-thousands" => s.show_thousands = value == "true",
                "show-zeroes" => s.show_zeroes = value == "true",
                "number-format" => s.number_format = NumberFormatSetting::parse_str(value),
                "angle-units" => s.angle_units = AngleSetting::parse_str(value),
                "refresh-interval" => {
                    if let Ok(v) = value.parse() {
                        s.refresh_interval = v;
                    }
                }
                "currency-display" => s.currency_display = CurrencyDisplay::parse_str(value),
                "enabled-completions" => {
                    s.enabled_completions = split_list(value);
                }
                "favorite-currencies" => {
                    s.favorite_currencies = split_list(value);
                }
                "button-mode" => s.button_mode = ButtonMode::parse_str(value),
                "unit-category" => s.unit_category = value.to_string(),
                "source-units" => s.source_units = split_list(value),
                "target-units" => s.target_units = split_list(value),
                "precision" => {
                    if let Ok(v) = value.parse() {
                        s.precision = v;
                    }
                }
                "window-maximized" => s.window_maximized = value == "true",
                "window-size" => {
                    if let Some((w, h)) = value.split_once(',') {
                        if let (Ok(w), Ok(h)) = (w.trim().parse(), h.trim().parse()) {
                            s.window_size = (w, h);
                        }
                    }
                }
                _ => {}
            }
        }

        // Second pass for the history section.
        let mut entries: BTreeMap<u32, HistoryEntry> = BTreeMap::new();
        let mut in_history = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_history = line == "[history]";
                continue;
            }
            if !in_history {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let Some((index, field)) = key.trim().split_once('.') else {
                continue;
            };
            let Ok(index) = index.trim().parse::<u32>() else {
                continue;
            };
            let entry = entries.entry(index).or_insert_with(|| HistoryEntry {
                equation: String::new(),
                answer: String::new(),
            });
            match field.trim() {
                "equation" => entry.equation = unescape_value(value),
                "answer" => entry.answer = unescape_value(value),
                _ => {}
            }
        }
        if !entries.is_empty() {
            s.history_tape = entries.into_values().collect();
        }
        s
    }

    fn save(&self, settings: &Settings) {
        let mut out = String::new();
        out.push_str("[settings]\n");
        let w = |out: &mut String, key: &str, value: String| {
            out.push_str(key);
            out.push('=');
            out.push_str(&value);
            out.push('\n');
        };
        w(&mut out, "accuracy", settings.accuracy.to_string());
        w(&mut out, "word-size", settings.word_size.to_string());
        w(&mut out, "base", settings.base.to_string());
        w(
            &mut out,
            "show-thousands",
            settings.show_thousands.to_string(),
        );
        w(&mut out, "show-zeroes", settings.show_zeroes.to_string());
        w(
            &mut out,
            "number-format",
            settings.number_format.as_str().to_string(),
        );
        w(
            &mut out,
            "angle-units",
            settings.angle_units.as_str().to_string(),
        );
        w(
            &mut out,
            "refresh-interval",
            settings.refresh_interval.to_string(),
        );
        w(
            &mut out,
            "currency-display",
            settings.currency_display.as_str().to_string(),
        );
        w(
            &mut out,
            "enabled-completions",
            join_list(&settings.enabled_completions),
        );
        w(
            &mut out,
            "favorite-currencies",
            join_list(&settings.favorite_currencies),
        );
        w(
            &mut out,
            "button-mode",
            settings.button_mode.as_str().to_string(),
        );
        w(&mut out, "unit-category", settings.unit_category.clone());
        w(&mut out, "source-units", join_list(&settings.source_units));
        w(&mut out, "target-units", join_list(&settings.target_units));
        w(&mut out, "precision", settings.precision.to_string());
        w(
            &mut out,
            "window-maximized",
            settings.window_maximized.to_string(),
        );
        w(
            &mut out,
            "window-size",
            format!("{},{}", settings.window_size.0, settings.window_size.1),
        );
        out.push('\n');
        out.push_str("[history]\n");
        for (i, e) in settings.history_tape.iter().enumerate() {
            // Equations are single-line (paste converts newlines to spaces,
            // doc 06); strip anything that would break the line format.
            let eq = flatten_value(&e.equation);
            let ans = flatten_value(&e.answer);
            out.push_str(&format!("{}.equation={}\n", i + 1, eq));
            out.push_str(&format!("{}.answer={}\n", i + 1, ans));
        }
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("ini.tmp");
        if std::fs::write(&tmp, out).is_ok() {
            let _ = std::fs::rename(&tmp, &self.path);
        }
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn join_list(items: &[String]) -> String {
    items.join(",")
}

/// Escape a value for single-line keyfile storage (`\n` → `\s`, `\` → `\\`).
fn flatten_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' => out.push_str("\\s"),
            c => out.push(c),
        }
    }
    out
}

/// Inverse of [`flatten_value`].
fn unescape_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => out.push(' '),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_schema() {
        let s = Settings::default();
        assert_eq!(s.accuracy, 9);
        assert_eq!(s.word_size, 64);
        assert_eq!(s.base, 10);
        assert!(!s.show_thousands);
        assert_eq!(s.refresh_interval, 604_800);
        assert_eq!(s.unit_category, "angle");
        assert_eq!(s.source_units, vec!["degree"]);
        assert_eq!(s.target_units, vec!["radian"]);
        assert_eq!(s.precision, 2000);
    }

    #[test]
    fn keyfile_round_trip() {
        let mut s = Settings {
            history_tape: vec![
                HistoryEntry {
                    equation: "2+2".into(),
                    answer: "4".into(),
                },
                HistoryEntry {
                    equation: "100+10%".into(),
                    answer: "110".into(),
                },
            ],
            ..Settings::default()
        };
        s.accuracy = 5;
        s.show_thousands = true;
        s.number_format = NumberFormatSetting::Engineering;
        s.angle_units = AngleSetting::Gradians;
        s.button_mode = ButtonMode::Programming;
        s.base = 16;
        s.favorite_currencies = vec!["USD".into(), "EUR".into()];
        s.window_size = (800, 600);
        let dir = std::env::temp_dir().join(format!("radix-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = KeyfileStore {
            path: dir.join("settings.ini"),
        };
        store.save(&s);
        let loaded = store.load();
        assert_eq!(loaded, s);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_garbage_yields_defaults() {
        let dir = std::env::temp_dir().join(format!("radix-test-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.ini");
        std::fs::write(&path, "this is not an ini\nword-size=999\nbase=0\n").unwrap();
        let s = KeyfileStore { path: path.clone() }.load();
        assert_eq!(s.word_size, 64); // clamped
        assert_eq!(s.base, 2); // clamped
        assert_eq!(s.accuracy, 9); // default
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
