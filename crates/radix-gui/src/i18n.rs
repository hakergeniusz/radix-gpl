//! Translations (D33): a minimal gettext reader. Msgids are the English
//! UI strings (the behavioral spec text), catalogs are the committed
//! `<lang>.po` compiled to `<lang>/LC_MESSAGES/radix.mo`.
//!
//! glibc's dgettext only honors the LANGUAGE chain once a *generated*
//! locale is set (de_DE.UTF-8 …) — dev machines and CI images typically
//! only have C/en_US, which would silently disable every translation.
//! The .mo format is trivial (big-endian offset tables, NUL-terminated
//! UTF-8 strings), so the lookup is done here directly: same catalogs,
//! same semantics, no locale generation required.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The translation domain — the .mo file name.
pub const DOMAIN: &str = "radix";

static CATALOG: OnceLock<Option<HashMap<String, String>>> = OnceLock::new();

/// Translate a (static or dynamic) msgid. Dynamic core-origin strings
/// (unit/category/function-description names) use the exact English
/// literal from `radix-core` as the msgid; untranslated entries return
/// the msgid unchanged.
pub fn gettext(s: &str) -> String {
    match CATALOG.get_or_init(load) {
        Some(map) => map.get(s).cloned().unwrap_or_else(|| s.to_string()),
        None => s.to_string(),
    }
}

/// Eagerly resolve and load the catalog (called before GTK init; loading
/// is lazy-safe either way).
pub fn init() {
    let _ = CATALOG.get_or_init(load);
}

/// gettext's language chain: LANGUAGE (a colon-separated priority list)
/// wins; otherwise the first non-empty one of LC_ALL, LC_MESSAGES, LANG.
/// Encoding and modifier suffixes are stripped (de_DE.UTF-8 → de_DE, de).
fn language_chain() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |value: &str, out: &mut Vec<String>| {
        for part in value.split(':') {
            let p = part.trim();
            if p.is_empty() || p == "C" || p == "POSIX" {
                continue;
            }
            let base = p.split('.').next().unwrap_or(p);
            let base = base.split('@').next().unwrap_or(base);
            if !base.is_empty() && !out.iter().any(|e| e == base) {
                out.push(base.to_string());
            }
        }
    };
    if let Ok(v) = std::env::var("LANGUAGE") {
        push(&v, &mut out);
    }
    if out.is_empty() {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                push(&v, &mut out);
                if !out.is_empty() {
                    break;
                }
            }
        }
    }
    out
}

fn load() -> Option<HashMap<String, String>> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::var_os("RADIX_LOCALEDIR") {
        dirs.push(d.into());
    }
    dirs.push("/app/share/locale".into());
    dirs.push("/usr/share/locale".into());
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/locales"));
    }
    for lang in language_chain() {
        for dir in &dirs {
            let mo = dir.join(&lang).join("LC_MESSAGES").join(format!("{DOMAIN}.mo"));
            if let Some(map) = parse_mo(&mo) {
                #[cfg(debug_assertions)]
                eprintln!("i18n: {} from {}", lang, mo.display());
                return Some(map);
            }
        }
    }
    #[cfg(debug_assertions)]
    eprintln!("i18n: no catalog for {:?} — English", language_chain());
    None
}

fn cstr(data: &[u8], off: usize, len: usize) -> Option<&str> {
    let raw = data.get(off..off + len)?;
    let head = raw.split(|&b| b == 0).next()?;
    std::str::from_utf8(head).ok()
}

fn parse_mo(path: &Path) -> Option<HashMap<String, String>> {
    let data = std::fs::read(path).ok()?;
    // The magic 0x950412de may be stored in either byte order; the rest of
    // the file follows the header's order.
    let bytes = |off: usize| -> Option<[u8; 4]> {
        data.get(off..off + 4).map(|s| [s[0], s[1], s[2], s[3]])
    };
    let magic = bytes(0)?;
    let rd: Box<dyn Fn(usize) -> Option<u32>> = if u32::from_be_bytes(magic) == 0x9504_12de {
        Box::new(move |off| bytes(off).map(u32::from_be_bytes))
    } else if u32::from_le_bytes(magic) == 0x9504_12de {
        Box::new(move |off| bytes(off).map(u32::from_le_bytes))
    } else {
        return None;
    };
    // Header: magic, version, N, original-table offset, translation-table
    // offset, hash-table size, hash-table offset.
    let n = rd(8)? as usize;
    let o_orig = rd(12)? as usize;
    let o_trans = rd(16)? as usize;
    let mut map = HashMap::with_capacity(n);
    for i in 0..n {
        let id = cstr(&data, rd(o_orig + i * 8 + 4)? as usize, rd(o_orig + i * 8)? as usize)?;
        let s = cstr(&data, rd(o_trans + i * 8 + 4)? as usize, rd(o_trans + i * 8)? as usize)?;
        if !id.is_empty() {
            map.insert(id.to_string(), s.to_string());
        }
    }
    Some(map)
}
