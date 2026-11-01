//! Registers (user variables) and custom-function files
//! (spec: doc 08 §2, decision D13 — Radix paths under `$XDG_DATA_HOME/radix/`).
//!
//! - `registers`: `name=value` lines; value rendered scientific, radix `.`,
//!   ~50 fractional digits, `×10¹²`-style superscript exponent
//!   ([`Serializer::register_string`]).
//! - `custom-functions`: `name(p1;p2)=expr@description` lines with the body
//!   text exactly as defined.
//!
//! Both files are re-saved whole on every set/delete.

use crate::eval::{EvalState, UserFunction};
use crate::serializer::Serializer;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `$XDG_DATA_HOME/radix/` (XDG base dir spec, default `~/.local/share`).
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty() && PathBuf::from(v).is_absolute())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("radix")
}

pub fn registers_path() -> PathBuf {
    data_dir().join("registers")
}

pub fn custom_functions_path() -> PathBuf {
    data_dir().join("custom-functions")
}

/// One parsed custom-function file line.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body_source: String,
    pub description: Option<String>,
}

/// Serialize the user variables of `state` to the registers file,
/// sorted by name for deterministic output.
pub fn save_registers(path: &Path, state: &EvalState) -> std::io::Result<()> {
    let mut lines: BTreeMap<&str, String> = BTreeMap::new();
    for (name, value) in &state.variables {
        lines.insert(name.as_str(), Serializer::register_string(value));
    }
    let mut out = String::new();
    for (name, value) in lines {
        out.push_str(name);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    write_atomic(path, out)
}

/// Parse a registers file into `name → value` pairs. Lines that fail to
/// parse are skipped (a corrupt line must not lose the others).
pub fn load_registers(path: &Path) -> Vec<(String, crate::number::Number)> {
    let mut out = Vec::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return out;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphabetic()) {
            continue;
        }
        if let Ok(num) = crate::solve::parse_value_string(value.trim()) {
            out.push((name.to_string(), num));
        }
    }
    out
}

/// Merge the registers file into `state` (names already defined in the
/// session win — the file is a snapshot from previous runs).
pub fn load_registers_into(path: &Path, state: &mut EvalState) {
    for (name, value) in load_registers(path) {
        state.variables.entry(name).or_insert(value);
    }
}

/// Serialize the user functions of `state` to the custom-functions file,
/// sorted by name for deterministic output.
pub fn save_functions(path: &Path, state: &EvalState) -> std::io::Result<()> {
    let mut funcs: Vec<&UserFunction> = state.functions.values().collect();
    funcs.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = String::new();
    for f in funcs {
        out.push_str(&f.name);
        out.push('(');
        out.push_str(&f.params.join(";"));
        out.push_str(")=");
        out.push_str(&flatten(&f.body_source));
        if let Some(d) = &f.description {
            out.push('@');
            out.push_str(&flatten(d));
        }
        out.push('\n');
    }
    write_atomic(path, out)
}

/// Load custom-function definitions (raw lines) from the file.
pub fn load_functions(path: &Path) -> Vec<StoredFunction> {
    let mut out = Vec::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return out;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(f) = parse_function_line(line) {
            out.push(f);
        }
    }
    out
}

/// Parse one `name(p1;p2)=expr[@description]` line.
pub fn parse_function_line(line: &str) -> Option<StoredFunction> {
    let open = line.find('(')?;
    let close = line.find(')')?;
    if close < open {
        return None;
    }
    let name = line[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphabetic()) {
        return None;
    }
    let params: Vec<String> = line[open + 1..close]
        .split(';')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let rest = line[close + 1..].trim();
    let rest = rest.strip_prefix('=')?;
    let (body, description) = match rest.split_once('@') {
        Some((b, d)) => (b.trim().to_string(), Some(d.trim().to_string())),
        None => (rest.trim().to_string(), None),
    };
    if body.is_empty() {
        return None;
    }
    Some(StoredFunction {
        name: name.to_string(),
        params,
        body_source: body,
        description,
    })
}

/// Rebuild a definition line from a stored function (doc 08 §2 format).
pub fn function_line(f: &StoredFunction) -> String {
    let mut s = format!("{}({})={}", f.name, f.params.join(";"), f.body_source);
    if let Some(d) = &f.description {
        s.push('@');
        s.push_str(d);
    }
    s
}

/// Strip line-breaking characters so a definition stays one file line.
fn flatten(s: &str) -> String {
    s.chars().filter(|c| *c != '\n' && *c != '\r').collect()
}

// ----------------------------------------------------------------------
// Currency provider cache (doc 08 §2): the raw provider downloads are
// cached under `$XDG_CACHE_HOME/radix/` and re-parsed on startup when the
// network is unavailable (provider format drift is a provider concern,
// OQ-06 — the cache stores the download verbatim).
// ----------------------------------------------------------------------

/// A currency rate provider and its cache file (doc 04 §3.2, doc 08 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Ims,
    Unt,
    Ecb,
    Boc,
}

impl Provider {
    /// Attempt order (doc 04 §3.2): IMF, UN, ECB, Bank of Canada.
    pub const ALL: [Provider; 4] = [Provider::Ims, Provider::Unt, Provider::Ecb, Provider::Boc];

    pub fn cache_file_name(&self) -> &'static str {
        match self {
            Self::Ims => "rms_five.xls",
            Self::Unt => "un-daily.xls",
            Self::Ecb => "eurofxref-daily.xml",
            Self::Boc => "fxtwdcad.xml",
        }
    }
}

/// `$XDG_CACHE_HOME/radix/` (default `~/.cache/radix`).
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty() && PathBuf::from(v).is_absolute())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("radix")
}

pub fn provider_cache_path(provider: Provider) -> PathBuf {
    cache_dir().join(provider.cache_file_name())
}

/// Store a raw provider download in the cache (best effort).
pub fn store_provider_cache(provider: Provider, text: &str) -> std::io::Result<()> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(provider_cache_path(provider), text)
}

fn write_atomic(path: &Path, contents: String) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyTable;
    use crate::eval::{EvalConfig, EvalState};
    use crate::numfuncs::AngleUnit;
    use crate::solve::solve;
    use crate::units::UnitCatalog;

    fn solved(state: &mut EvalState, input: &str) {
        let config = EvalConfig {
            precision: 2000,
            wordlen: 64,
            angle_unit: AngleUnit::Degrees,
            base: 10,
        };
        let units = UnitCatalog::new();
        let currencies = CurrencyTable::new();
        let out = solve(input, &config, state, &units, &currencies, None);
        assert!(out.error.is_none(), "{input} errored: {:?}", out.error);
    }

    #[test]
    fn registers_round_trip_scientific_format() {
        let mut state = EvalState::default();
        solved(&mut state, "half=0.5");
        solved(&mut state, "big=2^70");
        let dir = std::env::temp_dir().join(format!("radix-reg-{}", std::process::id()));
        let path = dir.join("registers");
        save_registers(&path, &state).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        // doc 08 §2: scientific format, radix '.', superscript exponent.
        assert!(text.contains("half=5×10⁻¹"), "got: {text}");
        assert!(text.contains("big=1."), "got: {text}"); // 2^70 ≈ 1.18×10²¹
        let mut fresh = EvalState::default();
        load_registers_into(&path, &mut fresh);
        let half = fresh.variables.get("half").unwrap();
        assert_eq!(Serializer::register_string(half), "5×10⁻¹");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn functions_round_trip_with_description() {
        let mut state = EvalState::default();
        solved(&mut state, "f(x;y;z)=x+y+z @ Sum of three");
        solved(&mut state, "g(x)=x^2");
        let dir = std::env::temp_dir().join(format!("radix-fn-{}", std::process::id()));
        let path = dir.join("custom-functions");
        save_functions(&path, &state).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("f(x;y;z)=x+y+z@Sum of three"), "got: {text}");
        assert!(text.contains("g(x)=x^2"), "got: {text}");
        let loaded = load_functions(&path);
        assert_eq!(loaded.len(), 2);
        let f = loaded.iter().find(|s| s.name == "f").unwrap();
        assert_eq!(f.params, vec!["x", "y", "z"]);
        assert_eq!(f.body_source, "x+y+z");
        assert_eq!(f.description.as_deref(), Some("Sum of three"));
        // The stored line re-parses through the normal definition path.
        let line = function_line(f);
        let mut fresh = EvalState::default();
        solved(&mut fresh, &line);
        assert!(fresh.functions.contains_key("f"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn corrupt_register_lines_are_skipped() {
        let dir = std::env::temp_dir().join(format!("radix-badreg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("registers");
        std::fs::write(&path, "good=2\nbroken !!!\n=3\nbad=not a number\n").unwrap();
        let regs = load_registers(&path);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].0, "good");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
