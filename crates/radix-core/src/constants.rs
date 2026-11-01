//! Built-in constants (spec: doc 04 §2).
//!
//! Constants are variables that always exist and cannot be reassigned
//! (assignment is silently ignored). Names are lowercase-ASCII or exact
//! Unicode symbols, including the Cyrillic `е` elementary-charge quirk.

use crate::numfuncs::pi_const;
use rug::{Float, Integer, Rational};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantCategory {
    Mathematics,
    Electromagnetism,
    AtomicNuclear,
    Thermodynamics,
    Gravitation,
}

#[derive(Debug, Clone, Copy)]
pub struct ConstantDef {
    /// Exact recognized symbol.
    pub symbol: &'static str,
    pub name: &'static str,
    pub category: ConstantCategory,
}

pub const CONSTANTS: &[ConstantDef] = &[
    ConstantDef {
        symbol: "e",
        name: "Euler’s Number",
        category: ConstantCategory::Mathematics,
    },
    ConstantDef {
        symbol: "π",
        name: "Pi",
        category: ConstantCategory::Mathematics,
    },
    ConstantDef {
        symbol: "τ",
        name: "Tau",
        category: ConstantCategory::Mathematics,
    },
    ConstantDef {
        symbol: "φ",
        name: "Golden Ratio",
        category: ConstantCategory::Mathematics,
    },
    ConstantDef {
        symbol: "c₀",
        name: "Light Speed",
        category: ConstantCategory::Electromagnetism,
    },
    // The elementary charge symbol is the CYRILLIC е (U+0435) — long-standing
    // quirk preserved (doc 04 §2).
    ConstantDef {
        symbol: "е",
        name: "Elementary Charge",
        category: ConstantCategory::Electromagnetism,
    },
    ConstantDef {
        symbol: "Z₀",
        name: "Impedance of Vacuum",
        category: ConstantCategory::Electromagnetism,
    },
    ConstantDef {
        symbol: "μ₀",
        name: "Permeability of Vacuum",
        category: ConstantCategory::Electromagnetism,
    },
    ConstantDef {
        symbol: "ε₀",
        name: "Permittivity of Vacuum",
        category: ConstantCategory::Electromagnetism,
    },
    ConstantDef {
        symbol: "h",
        name: "Planck’s Constant",
        category: ConstantCategory::AtomicNuclear,
    },
    ConstantDef {
        symbol: "α",
        name: "Fine-Structure Constant",
        category: ConstantCategory::AtomicNuclear,
    },
    ConstantDef {
        symbol: "mₑ",
        name: "Electron Mass",
        category: ConstantCategory::AtomicNuclear,
    },
    ConstantDef {
        symbol: "mₚ",
        name: "Proton Mass",
        category: ConstantCategory::AtomicNuclear,
    },
    ConstantDef {
        symbol: "k",
        name: "Boltzmann Constant",
        category: ConstantCategory::Thermodynamics,
    },
    ConstantDef {
        symbol: "mᵤ",
        name: "Atomic Mass Unit",
        category: ConstantCategory::Thermodynamics,
    },
    ConstantDef {
        symbol: "R",
        name: "Molar Gas Constant",
        category: ConstantCategory::Thermodynamics,
    },
    ConstantDef {
        symbol: "σ",
        name: "Stefan-Boltzmann Constant",
        category: ConstantCategory::Thermodynamics,
    },
    ConstantDef {
        symbol: "Nᴀ",
        name: "Avogadro’s Number",
        category: ConstantCategory::Thermodynamics,
    },
    ConstantDef {
        symbol: "G",
        name: "Gravitational Constant",
        category: ConstantCategory::Gravitation,
    },
    ConstantDef {
        symbol: "g",
        name: "Earth Acceleration",
        category: ConstantCategory::Gravitation,
    },
];

/// True when `name` is a built-in constant (case-sensitive; `pi`/`tau`/`i`
/// are lowercase-only, doc 02 §4).
pub fn is_constant(name: &str) -> bool {
    name == "pi" || name == "tau" || name == "i" || CONSTANTS.iter().any(|c| c.symbol == name)
}

/// Value of a built-in constant at the given precision (doc 04 §2 exact
/// stored values; decimal strings go through the exact rational path).
pub fn constant_value(name: &str, prec: u32) -> Option<crate::number::Number> {
    use crate::number::Number;
    let n = match name {
        "pi" | "π" => Number::float(pi_const(prec)),
        "tau" | "τ" => Number::float(pi_const(prec) * 2),
        "i" => {
            return Some(Number::complex(Float::new(prec), Float::with_val(prec, 1)));
        }
        "e" => Number::float(Float::with_val(prec, 1).exp()),
        "φ" => {
            // (1+√5)/2 at precision P.
            let five = Float::with_val(prec + 8, 5);
            let sqrt5 = five.sqrt();
            let phi = (sqrt5 + 1) / 2;
            Number::float(Float::with_val(prec, phi))
        }
        "c₀" => Number::integer(Integer::from(299792458), prec),
        "е" => Number::rational(parse_rat("0.0000000000000000001602176634")?, prec),
        "Z₀" => Number::rational(parse_rat("376.730313412")?, prec),
        "μ₀" => Number::rational(parse_rat("0.00000125663706127")?, prec),
        "ε₀" => Number::rational(parse_rat("0.0000000000088541878188")?, prec),
        "h" => Number::rational(
            parse_rat("0.000000000000000000000000000000000662607015")?,
            prec,
        ),
        "α" => Number::rational(parse_rat("0.0072973525643")?, prec),
        "mₑ" => Number::rational(
            parse_rat("0.00000000000000000000000000000091093837139")?,
            prec,
        ),
        "mₚ" => Number::rational(parse_rat("0.00000000000000000000000000167262192595")?, prec),
        "k" => Number::rational(parse_rat("0.00000000000000000000001380649")?, prec),
        "mᵤ" => Number::rational(parse_rat("0.00000000000000000000000000166053906892")?, prec),
        "R" => Number::rational(parse_rat("8.31446261815324")?, prec),
        "σ" => Number::rational(parse_rat("0.00000005670374419")?, prec),
        "Nᴀ" => Number::integer(Integer::from(602214076000000000000000u128), prec),
        "G" => Number::rational(parse_rat("0.000000000066743")?, prec),
        "g" => Number::rational(parse_rat("9.80665")?, prec),
        _ => return None,
    };
    Some(n)
}

fn parse_rat(s: &str) -> Option<Rational> {
    crate::number::parse_decimal_rational(s)
}
