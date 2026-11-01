//! Headless `--solve` path (doc 08 §4): no GTK initialization.

use radix_core::currency_fetch::{load_cached_rates, refresh_rates, RefreshOptions};
use radix_core::error::ErrorCode;
use radix_core::eval::{EvalConfig, EvalState};
use radix_core::numfuncs::AngleUnit;
use radix_core::serializer::{NumberFormat, Serializer};
use radix_core::settings::{AngleSetting, KeyfileStore, SettingsStore};
use radix_core::solve::solve;
use radix_core::units::UnitCatalog;
use std::io::Write;

/// Locale normalization for CLI input (doc 08 §4): thousands separator
/// removed (unless it is `.`), locale radix replaced by `.`. C locale (D14).
pub fn normalize_input(input: &str) -> String {
    // Escape sequences in --equation are unescaped first (doc 08 §4);
    // --solve takes the argument verbatim.
    input.chars().filter(|c| *c != ',').collect()
}

/// Unescape `\n`, `\t`, `\\` etc. for `--equation` (doc 08 §4).
// This module is included by two binaries (`radix` and the search provider);
// the provider only uses the normalization/evaluation core, so the
// --equation/--solve entry points are dead from its point of view.
#[allow(dead_code)]
pub fn unescape_argument(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

// Shared with the search provider's query evaluation.
#[allow(dead_code)]
fn angle_from_settings() -> AngleUnit {
    let s = KeyfileStore::new().load();
    match s.angle_units {
        AngleSetting::Radians => AngleUnit::Radians,
        AngleSetting::Gradians => AngleUnit::Gradians,
        AngleSetting::Degrees => AngleUnit::Degrees,
    }
}

/// Evaluate and print per doc 08 §4: AUTOMATIC format, accuracy 9,
/// representation base applied; `Error: <msg>` for MP, `Error: ErrorCode.X`
/// otherwise, exit 1. UNKNOWN_RATE triggers a synchronous force refresh and
/// one retry.
// Entry point of the `radix` binary's headless mode; dead in the provider.
#[allow(dead_code)]
pub fn run_solve(equation: &str) -> std::process::ExitCode {
    let canonical = normalize_input(equation);
    let config = EvalConfig {
        precision: 1000, // library default in this path (doc 08 §4)
        wordlen: 32,
        angle_unit: angle_from_settings(),
        base: 10,
    };
    let mut state = EvalState::default();
    let units = UnitCatalog::new();
    let currencies = load_cached_rates();
    let out = solve(&canonical, &config, &mut state, &units, &currencies, None);
    let out = match out.error.as_ref().map(|e| e.code) {
        Some(ErrorCode::UnknownRate) => {
            let refreshed = refresh_rates(RefreshOptions {
                interval_secs: 604_800,
                force: true,
            });
            solve(&canonical, &config, &mut state, &units, &refreshed, None)
        }
        _ => out,
    };
    if let Some(e) = out.error {
        if e.code == ErrorCode::Mp {
            eprintln!("Error: {}", e.message);
        } else {
            eprintln!("Error: ErrorCode.{}", e.code.name());
        }
        return std::process::ExitCode::from(1);
    }
    let Some(v) = out.value else {
        return std::process::ExitCode::from(1);
    };
    let mut ser = Serializer {
        format: NumberFormat::Automatic,
        accuracy: 9,
        show_thousands: false,
        ..Serializer::default()
    };
    // Base conversions override the display base (CLI-T004: `10 in hex`).
    ser.repr_base = if out.repr_base == 0 {
        config.base
    } else {
        out.repr_base
    };
    match ser.serialize(&v) {
        Ok(s) => {
            println!("{s}");
            let _ = std::io::stdout().flush();
            std::process::ExitCode::from(0)
        }
        Err(e) => {
            // Output-formatting errors go to stderr (doc 08 §4).
            eprintln!("Error: {}", e.message);
            std::process::ExitCode::from(1)
        }
    }
}
