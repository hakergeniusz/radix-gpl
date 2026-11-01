//! Unit catalog and conversion (spec: doc 04 §3.1).
//!
//! Each unit converts to its category base unit; "×N" factors multiply,
//! "÷N" factors divide (the doc's convention — e.g. rack-unit `÷22.497…`
//! means to_base = x/22.497… = 0.04445 m per U, verified by the catalog
//! tests 44.45 mm in U = 1 and 1 U in inch = 1.75).

use crate::error::{messages, Error, ErrorCode};
use crate::number::Number;
use rug::{Integer, Rational};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitDef {
    /// Category key (doc 04 §3.1; Mass uses key "weight").
    pub category: &'static str,
    pub name: &'static str,
    pub symbols: &'static [&'static str],
    pub kind: UnitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// to_base = x × factor (factor is an exact rational expression string,
    /// possibly "a/b" with decimal parts).
    Factor(&'static str),
    /// to_base = x × π × num / den (angle units).
    Angle(i64, i64),
    /// Temperature offset conversions.
    Temp(TempScale),
    /// Number-base pseudo units (converter UI only).
    BaseUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempScale {
    Kelvin,
    Celsius,
    Fahrenheit,
    Rankine,
}

macro_rules! u {
    ($cat:expr, $name:expr, $symbols:expr, Factor($f:expr)) => {
        UnitDef {
            category: $cat,
            name: $name,
            symbols: $symbols,
            kind: UnitKind::Factor($f),
        }
    };
    ($cat:expr, $name:expr, $symbols:expr, Angle($n:expr, $d:expr)) => {
        UnitDef {
            category: $cat,
            name: $name,
            symbols: $symbols,
            kind: UnitKind::Angle($n, $d),
        }
    };
    ($cat:expr, $name:expr, $symbols:expr, Temp($t:expr)) => {
        UnitDef {
            category: $cat,
            name: $name,
            symbols: $symbols,
            kind: UnitKind::Temp($t),
        }
    };
}

pub const UNITS: &[UnitDef] = &[
    // ---- Angle (base: radian) ------------------------------------------
    u!(
        "angle",
        "Degrees (Decimal)",
        &["degree", "degrees", "deg"],
        Angle(1, 180)
    ),
    u!("angle", "Degrees (D°M′S″)", &[], Angle(1, 180)),
    u!(
        "angle",
        "Radians",
        &["radian", "radians", "rad"],
        Angle(1, 1)
    ),
    u!(
        "angle",
        "Gradians",
        &["gradian", "gradians", "grad"],
        Angle(1, 200)
    ),
    // ---- Length (base: meter) --------------------------------------------
    u!(
        "length",
        "Parsec",
        &["parsec", "parsecs", "pc"],
        Factor("30857000000000000")
    ),
    u!(
        "length",
        "Lightyear",
        &["lightyear", "lightyears", "ly"],
        Factor("9460730472580800")
    ),
    u!(
        "length",
        "Astronomical Unit",
        &["astronomical-unit", "astronomical-units", "au"],
        Factor("149597870700")
    ),
    u!(
        "length",
        "Rack Unit",
        &["rack-unit", "rack-units", "U"],
        Factor("1/22.49718785151856")
    ),
    u!(
        "length",
        "Nautical Mile",
        &["nautical-mile", "nautical-miles", "nmi"],
        Factor("1852")
    ),
    u!(
        "length",
        "Mile",
        &["mile", "miles", "mi"],
        Factor("1609.344")
    ),
    u!(
        "length",
        "Kilometer",
        &["kilometer", "kilometers", "km", "kms"],
        Factor("1000")
    ),
    u!(
        "length",
        "Cable",
        &["cable", "cables", "cb"],
        Factor("219.456")
    ),
    u!(
        "length",
        "Fathom",
        &["fathom", "fathoms", "ftm"],
        Factor("1.8288")
    ),
    u!("length", "Meter", &["meter", "meters", "m"], Factor("1")),
    u!("length", "Yard", &["yard", "yards", "yd"], Factor("0.9144")),
    u!("length", "Foot", &["foot", "feet", "ft"], Factor("0.3048")),
    u!(
        "length",
        "Inch",
        &["inch", "inches", "in"],
        Factor("0.0254")
    ),
    u!(
        "length",
        "Centimeter",
        &["centimeter", "centimeters", "cm", "cms"],
        Factor("1/100")
    ),
    u!(
        "length",
        "Millimeter",
        &["millimeter", "millimeters", "mm"],
        Factor("1/1000")
    ),
    u!(
        "length",
        "Micrometer",
        &["micrometer", "micrometers", "um"],
        Factor("1/1000000")
    ),
    u!(
        "length",
        "Nanometer",
        &["nanometer", "nanometers", "nm"],
        Factor("1/1000000000")
    ),
    u!(
        "length",
        "Point",
        &["point", "points", "pt", "pts"],
        Factor("0.000352777778")
    ),
    // ---- Area (base: square-meter) ---------------------------------------
    u!(
        "area",
        "Hectare",
        &["hectare", "hectares", "ha"],
        Factor("10000")
    ),
    u!("area", "Acre", &["acre", "acres"], Factor("4046.8564224")),
    u!(
        "area",
        "Square Mile",
        &["square-mile", "square-miles", "mi²"],
        Factor("2589988.110336")
    ),
    u!(
        "area",
        "Square Foot",
        &["square-foot", "square-feet", "ft²"],
        Factor("1/10.763910417")
    ),
    u!(
        "area",
        "Square Kilometer",
        &["square-kilometer", "square-kilometers", "km²"],
        Factor("1000000")
    ),
    u!(
        "area",
        "Square Meter",
        &["square-meter", "square-meters", "m²"],
        Factor("1")
    ),
    u!(
        "area",
        "Square Centimeter",
        &["square-centimeter", "square-centimeters", "cm²"],
        Factor("0.0001")
    ),
    u!(
        "area",
        "Square Millimeter",
        &["square-millimeter", "square-millimeters", "mm²"],
        Factor("0.000001")
    ),
    // ---- Volume (base: litre) ---------------------------------------------
    u!(
        "volume",
        "Cubic Meter",
        &["cubic-meter", "cubic-meters", "m³"],
        Factor("1000")
    ),
    u!(
        "volume",
        "Imperial Gallon",
        &[
            "imperial-gallon",
            "imperial-gallons",
            "imp_gallon",
            "imp_gallons",
            "imp_gal"
        ],
        Factor("4.54609")
    ),
    u!(
        "volume",
        "US Gallon",
        &["gallon", "gallons", "gal"],
        Factor("3.785412")
    ),
    u!(
        "volume",
        "Litre",
        &["litre", "litres", "liter", "liters", "L"],
        Factor("1")
    ),
    u!(
        "volume",
        "Quart",
        &["quart", "quarts", "qt"],
        Factor("0.9463529")
    ),
    u!(
        "volume",
        "Imperial Pint",
        &[
            "imperial-pint",
            "imperial-pints",
            "imp_pt",
            "imp_pint",
            "imp_pints"
        ],
        Factor("0.56826125")
    ),
    u!(
        "volume",
        "US Pint",
        &["pint", "pints", "pt"],
        Factor("0.4731765")
    ),
    u!("volume", "Cup", &["cup", "cups", "cp"], Factor("0.25")),
    u!(
        "volume",
        "Millilitre",
        &[
            "millilitre",
            "millilitres",
            "milliliter",
            "milliliters",
            "mL",
            "cm³"
        ],
        Factor("0.001")
    ),
    u!(
        "volume",
        "Microlitre",
        &[
            "microlitre",
            "microlitres",
            "microliter",
            "microliters",
            "μL",
            "uL",
            "mm³"
        ],
        Factor("0.000001")
    ),
    // ---- Mass (category key "weight", base: kilograms) ---------------------
    u!("weight", "Tonne", &["tonne", "tonnes"], Factor("1000")),
    u!(
        "weight",
        "Kilograms",
        &["kilogram", "kilograms", "kg", "kgs"],
        Factor("1")
    ),
    u!(
        "weight",
        "Pound",
        &["pound", "pounds", "lb", "lbs"],
        Factor("0.45359237")
    ),
    u!(
        "weight",
        "Ounce",
        &["ounce", "ounces", "oz"],
        Factor("0.02834952")
    ),
    u!(
        "weight",
        "Troy Ounce",
        &["troy-ounce", "troy-ounces", "ozt"],
        Factor("0.0311034768")
    ),
    u!("weight", "Gram", &["gram", "grams", "g"], Factor("0.001")),
    u!(
        "weight",
        "Milligram",
        &["milligram", "milligrams", "mg"],
        Factor("0.000001")
    ),
    u!(
        "weight",
        "Microgram",
        &["microgram", "micrograms", "μg", "ug"],
        Factor("0.000000001")
    ),
    u!(
        "weight",
        "Stone",
        &["stone", "stones", "st"],
        Factor("6.350293")
    ),
    // ---- Duration (base: second) --------------------------------------------
    u!(
        "duration",
        "Century",
        &["century", "centuries"],
        Factor("3155760000")
    ),
    u!(
        "duration",
        "Decade",
        &["decade", "decades"],
        Factor("315576000")
    ),
    u!("duration", "Year", &["year", "years"], Factor("31557600")),
    u!("duration", "Month", &["month", "months"], Factor("2629800")),
    u!("duration", "Week", &["week", "weeks"], Factor("604800")),
    u!("duration", "Day", &["day", "days"], Factor("86400")),
    u!("duration", "Hour", &["hour", "hours"], Factor("3600")),
    u!("duration", "Minute", &["minute", "minutes"], Factor("60")),
    u!(
        "duration",
        "Second",
        &["second", "seconds", "s"],
        Factor("1")
    ),
    u!(
        "duration",
        "Millisecond",
        &["millisecond", "milliseconds", "ms"],
        Factor("0.001")
    ),
    u!(
        "duration",
        "Microsecond",
        &["microsecond", "microseconds", "us", "μs"],
        Factor("0.000001")
    ),
    u!(
        "duration",
        "Nanosecond",
        &["nanosecond", "nanoseconds", "ns"],
        Factor("0.000000001")
    ),
    // ---- Speed (base: meters-second) -----------------------------------------
    u!(
        "speed",
        "Kilometers per hour",
        &["kilometers-hour", "kmph", "kmh", "kph"],
        Factor("1/3.6")
    ),
    u!(
        "speed",
        "Miles per hour",
        &["miles-hour", "miph", "mph", "mi/h"],
        Factor("1/2.23693629")
    ),
    u!(
        "speed",
        "Meters per second",
        &["meters-second", "mps"],
        Factor("1")
    ),
    u!(
        "speed",
        "Feet per second",
        &["feet-second", "fps"],
        Factor("1/3.28084")
    ),
    u!(
        "speed",
        "Knot",
        &["knot", "knots", "kt", "kn", "nd"],
        Factor("1/1.94384449")
    ),
    // ---- Frequency (base: hertz) ----------------------------------------------
    u!("frequency", "Hertz", &["hertz", "Hz"], Factor("1")),
    u!(
        "frequency",
        "Kilohertz",
        &["kilohertz", "kHz"],
        Factor("1000")
    ),
    u!(
        "frequency",
        "Megahertz",
        &["megahertz", "MHz"],
        Factor("1000000")
    ),
    u!(
        "frequency",
        "Gigahertz",
        &["gigahertz", "GHz"],
        Factor("1000000000")
    ),
    u!(
        "frequency",
        "Terahertz",
        &["terahertz", "THz"],
        Factor("1000000000000")
    ),
    // ---- Temperature (base: kelvin; OFFSET conversions) ------------------------
    u!(
        "temperature",
        "Degree Celsius",
        &[
            "degree-celsius",
            "degC",
            "°C",
            "˚C",
            "C",
            "c",
            "Celsius",
            "celsius"
        ],
        Temp(TempScale::Celsius)
    ),
    u!(
        "temperature",
        "Degree Fahrenheit",
        &[
            "degree-fahrenheit",
            "degF",
            "°F",
            "˚F",
            "F",
            "f",
            "Fahrenheit",
            "fahrenheit"
        ],
        Temp(TempScale::Fahrenheit)
    ),
    u!(
        "temperature",
        "Degree Kelvin",
        &["degree-kelvin", "k", "K", "Kelvin", "kelvin"],
        Temp(TempScale::Kelvin)
    ),
    u!(
        "temperature",
        "Degree Rankine",
        &[
            "degree-rankine",
            "degR",
            "°R",
            "˚R",
            "°Ra",
            "˚Ra",
            "r",
            "R",
            "Rankine",
            "rankine"
        ],
        Temp(TempScale::Rankine)
    ),
    // ---- Energy (base: joule) -----------------------------------------------------
    u!(
        "energy",
        "Joule",
        &["joule", "joules", "J", "j"],
        Factor("1")
    ),
    u!(
        "energy",
        "Kilojoule",
        &["kilojoule", "kilojoules", "KJ", "kJ"],
        Factor("1000")
    ),
    u!(
        "energy",
        "Megajoule",
        &["megajoule", "megajoules", "MJ"],
        Factor("1000000")
    ),
    u!(
        "energy",
        "Kilowatt hour",
        &["kilowatthour", "kilowatthours", "kwh", "kWh"],
        Factor("3600000")
    ),
    u!("energy", "BTU", &["btu", "BTU"], Factor("1054.350264489")),
    u!(
        "energy",
        "Calorie",
        &["calorie", "calories", "cal"],
        Factor("4.184")
    ),
    u!(
        "energy",
        "Kilocalorie",
        &["kilocalorie", "kilocalories", "kcal"],
        Factor("4184")
    ),
    u!("energy", "Erg", &["erg", "ergs"], Factor("1/10000000")),
    u!(
        "energy",
        "Electronvolt",
        &["ev", "eV", "electronvolt", "electronvolts"],
        Factor("1.602176634/10000000000000000000")
    ),
    u!(
        "energy",
        "Foot-pound",
        &["ftlb", "foot-pound", "foot-pounds", "ft-lb", "ft-lbs"],
        Factor("1.3558179483314004")
    ),
    // ---- Pressure (base: pascal) -----------------------------------------------------
    u!(
        "pressure",
        "Pascal",
        &["pascal", "pascals", "Pa"],
        Factor("1")
    ),
    u!(
        "pressure",
        "Kilopascal",
        &["kilopascal", "kilopascals", "kPa"],
        Factor("1000")
    ),
    u!(
        "pressure",
        "Hectopascal",
        &["hectopascal", "hectopascals", "hPa"],
        Factor("100")
    ),
    u!("pressure", "Bar", &["bar", "bars"], Factor("100000")),
    u!(
        "pressure",
        "Atmosphere",
        &["atm", "atmosphere", "atmospheres"],
        Factor("101325")
    ),
    u!(
        "pressure",
        "mmHg",
        &["mmhg", "mmHg"],
        Factor("133.322387415")
    ),
    u!("pressure", "inHg", &["inhg", "inHg"], Factor("3386.38")),
    u!("pressure", "psi", &["psi"], Factor("6894.759")),
    // ---- Digital Storage (category key "data", base: byte) ----------------------------
    u!("data", "Bit", &["bit", "bits", "b"], Factor("1/8")),
    u!("data", "Byte", &["byte", "bytes", "B"], Factor("1")),
    u!("data", "Nibble", &["nibble", "nibbles"], Factor("1/2")),
    u!(
        "data",
        "Kilobit",
        &["kilobit", "kilobits", "kb", "Kb"],
        Factor("1000/8")
    ),
    u!(
        "data",
        "Kilobyte",
        &["kilobyte", "kilobytes", "kB", "KB"],
        Factor("1000")
    ),
    u!(
        "data",
        "Kibibit",
        &["kibibit", "kibibits", "Kib"],
        Factor("1024/8")
    ),
    u!(
        "data",
        "Kibibyte",
        &["kibibyte", "kibibytes", "KiB"],
        Factor("1024")
    ),
    u!(
        "data",
        "Megabit",
        &["megabit", "megabits", "Mb"],
        Factor("1000000/8")
    ),
    u!(
        "data",
        "Megabyte",
        &["megabyte", "megabytes", "MB"],
        Factor("1000000")
    ),
    u!(
        "data",
        "Mebibit",
        &["mebibit", "mebibits", "Mib"],
        Factor("1048576/8")
    ),
    u!(
        "data",
        "Mebibyte",
        &["mebibyte", "mebibytes", "MiB"],
        Factor("1048576")
    ),
    u!(
        "data",
        "Gigabit",
        &["gigabit", "gigabits", "Gb"],
        Factor("1000000000/8")
    ),
    u!(
        "data",
        "Gigabyte",
        &["gigabyte", "gigabytes", "GB"],
        Factor("1000000000")
    ),
    u!(
        "data",
        "Gibibit",
        &["gibibit", "gibibits", "Gib"],
        Factor("1073741824/8")
    ),
    u!(
        "data",
        "Gibibyte",
        &["gibibyte", "gibibytes", "GiB"],
        Factor("1073741824")
    ),
    u!(
        "data",
        "Terabit",
        &["terabit", "terabits", "Tb"],
        Factor("1000000000000/8")
    ),
    u!(
        "data",
        "Terabyte",
        &["terabyte", "terabytes", "TB"],
        Factor("1000000000000")
    ),
    u!(
        "data",
        "Tebibit",
        &["tebibit", "tebibits", "Tib"],
        Factor("1099511627776/8")
    ),
    u!(
        "data",
        "Tebibyte",
        &["tebibyte", "tebibytes", "TiB"],
        Factor("1099511627776")
    ),
    u!(
        "data",
        "Petabit",
        &["petabit", "petabits", "Pb"],
        Factor("1000000000000000/8")
    ),
    u!(
        "data",
        "Petabyte",
        &["petabyte", "petabytes", "PB"],
        Factor("1000000000000000")
    ),
    u!(
        "data",
        "Pebibit",
        &["pebibit", "pebibits", "Pib"],
        Factor("1125899906842624/8")
    ),
    u!(
        "data",
        "Pebibyte",
        &["pebibyte", "pebibytes", "PiB"],
        Factor("1125899906842624")
    ),
    u!(
        "data",
        "Exabit",
        &["exabit", "exabits", "Eb"],
        Factor("1000000000000000000/8")
    ),
    u!(
        "data",
        "Exabyte",
        &["exabyte", "exabytes", "EB"],
        Factor("1000000000000000000")
    ),
    u!(
        "data",
        "Exbibit",
        &["exbibit", "exbibits", "Eib"],
        Factor("1152921504606846976/8")
    ),
    u!(
        "data",
        "Exbibyte",
        &["exbibyte", "exbibytes", "EiB"],
        Factor("1152921504606846976")
    ),
    u!(
        "data",
        "Zettabit",
        &["zettabit", "zettabits", "Zb"],
        Factor("1000000000000000000000/8")
    ),
    u!(
        "data",
        "Zettabyte",
        &["zettabyte", "zettabytes", "ZB"],
        Factor("1000000000000000000000")
    ),
    u!(
        "data",
        "Zebibit",
        &["zebibit", "zebibits", "Zib"],
        Factor("1180591620717411303424/8")
    ),
    u!(
        "data",
        "Zebibyte",
        &["zebibyte", "zebibytes", "ZiB"],
        Factor("1180591620717411303424")
    ),
    u!(
        "data",
        "Yottabit",
        &["yottabit", "yottabits", "Yb"],
        Factor("1000000000000000000000000/8")
    ),
    u!(
        "data",
        "Yottabyte",
        &["yottabyte", "yottabytes", "YB"],
        Factor("1000000000000000000000000")
    ),
    u!(
        "data",
        "Yobibit",
        &["yobibit", "yobibits", "Yib"],
        Factor("1208925819614629174706176/8")
    ),
    u!(
        "data",
        "Yobibyte",
        &["yobibyte", "yobibytes", "YiB"],
        Factor("1208925819614629174706176")
    ),
    // ---- Number Base (converter UI only) -------------------------------------------------
    UnitDef {
        category: "base",
        name: "Binary",
        symbols: &["2"],
        kind: UnitKind::BaseUnit,
    },
    UnitDef {
        category: "base",
        name: "Octal",
        symbols: &["8"],
        kind: UnitKind::BaseUnit,
    },
    UnitDef {
        category: "base",
        name: "Decimal",
        symbols: &["10"],
        kind: UnitKind::BaseUnit,
    },
    UnitDef {
        category: "base",
        name: "Hexadecimal",
        symbols: &["16"],
        kind: UnitKind::BaseUnit,
    },
];

/// Display order of categories (doc 04 §3.1).
pub const CATEGORY_ORDER: &[(&str, &str)] = &[
    ("angle", "Angle"),
    ("length", "Length"),
    ("area", "Area"),
    ("volume", "Volume"),
    ("weight", "Mass"),
    ("duration", "Duration"),
    ("speed", "Speed"),
    ("frequency", "Frequency"),
    ("temperature", "Temperature"),
    ("energy", "Energy"),
    ("pressure", "Pressure"),
    ("data", "Digital Storage"),
    ("currency", "Currency"),
    ("base", "Number Base"),
];

/// Parse a factor expression: optional "a/b" with decimal parts on both
/// sides; both sides exact.
pub fn parse_factor(s: &str) -> Rational {
    if let Some((a, b)) = s.split_once('/') {
        let na = crate::number::parse_decimal_rational(a).unwrap_or(Rational::from(1));
        let nb = crate::number::parse_decimal_rational(b).unwrap_or(Rational::from(1));
        na / nb
    } else {
        crate::number::parse_decimal_rational(s).unwrap_or(Rational::from(1))
    }
}

/// Resolved unit lookup with the ambiguity rule (doc 04 §3.1): a symbol
/// matching more than one unit resolves to no unit at all.
pub struct UnitCatalog {
    /// symbol → list of unit indices (case-sensitive exact).
    exact: std::collections::HashMap<String, Vec<usize>>,
    /// lowercased symbol → list of unit indices.
    insensitive: std::collections::HashMap<String, Vec<usize>>,
}

impl UnitCatalog {
    pub fn new() -> Self {
        let mut exact = std::collections::HashMap::new();
        let mut insensitive = std::collections::HashMap::new();
        for (idx, u) in UNITS.iter().enumerate() {
            for sym in u.symbols {
                exact
                    .entry(sym.to_string())
                    .or_insert_with(Vec::new)
                    .push(idx);
                insensitive
                    .entry(sym.to_lowercase())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }
        UnitCatalog { exact, insensitive }
    }

    /// Case-sensitive first, then case-insensitive; multiple matches within
    /// a pass → no unit (doc 02 §4, doc 04 §3.1).
    pub fn by_symbol(&self, symbol: &str) -> Option<&'static UnitDef> {
        for map in [&self.exact, &self.insensitive] {
            if let Some(v) = map.get(symbol) {
                if v.len() == 1 {
                    return Some(&UNITS[v[0]]);
                }
                if v.is_empty() {
                    continue;
                }
                // Ambiguous within this pass → no unit at all.
                return None;
            }
        }
        None
    }

    pub fn units_in_category(&self, category: &str) -> Vec<&'static UnitDef> {
        UNITS.iter().filter(|u| u.category == category).collect()
    }
}

impl Default for UnitCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Is `symbol` a known unit (single-match rule applied)?
pub fn is_unit_symbol(catalog: &UnitCatalog, symbol: &str) -> bool {
    catalog.by_symbol(symbol).is_some()
}

/// Convert a value between two units of the same category (doc 04 §3).
pub fn convert(value: &Number, from: &UnitDef, to: &UnitDef, prec: u32) -> Result<Number, Error> {
    if from.category != to.category {
        return Err(Error::with_message(
            ErrorCode::UnknownConversion,
            messages::unknown_conversion(),
        ));
    }
    let to_base = |v: &Number, u: &UnitDef| -> Result<Number, Error> {
        match u.kind {
            UnitKind::Factor(f) => Ok(v.mul(&Number::rational(parse_factor(f), prec))),
            UnitKind::Angle(n, d) => {
                let pi = Number::float(crate::numfuncs::pi_const(prec));
                Ok(v.mul(&pi)
                    .mul(&Number::int(n, prec))
                    .div(&Number::int(d, prec))?)
            }
            UnitKind::Temp(t) => temp_to_base(v, t, prec),
            UnitKind::BaseUnit => Ok(v.clone()),
        }
    };
    let from_base = |v: &Number, u: &UnitDef| -> Result<Number, Error> {
        match u.kind {
            UnitKind::Factor(f) => v.div(&Number::rational(parse_factor(f), prec)),
            UnitKind::Angle(n, d) => {
                let pi = Number::float(crate::numfuncs::pi_const(prec));
                Ok(v.mul(&Number::int(d, prec))
                    .div(&pi)?
                    .div(&Number::int(n, prec))?)
            }
            UnitKind::Temp(t) => temp_from_base(v, t, prec),
            UnitKind::BaseUnit => Ok(v.clone()),
        }
    };
    let base = to_base(value, from)?;
    from_base(&base, to)
}

fn temp_to_base(v: &Number, t: TempScale, prec: u32) -> Result<Number, Error> {
    match t {
        TempScale::Kelvin => Ok(v.clone()),
        TempScale::Celsius => Ok(v.add(&Number::rational(parse_factor("273.15"), prec))),
        TempScale::Fahrenheit => Ok(v
            .add(&Number::rational(parse_factor("459.67"), prec))
            .mul(&Number::rational(parse_factor("5/9"), prec))),
        TempScale::Rankine => Ok(v.mul(&Number::rational(parse_factor("5/9"), prec))),
    }
}

fn temp_from_base(v: &Number, t: TempScale, prec: u32) -> Result<Number, Error> {
    match t {
        TempScale::Kelvin => Ok(v.clone()),
        TempScale::Celsius => Ok(v.sub(&Number::rational(parse_factor("273.15"), prec))),
        TempScale::Fahrenheit => Ok(v
            .mul(&Number::rational(parse_factor("9/5"), prec))
            .sub(&Number::rational(parse_factor("459.67"), prec))),
        TempScale::Rankine => Ok(v.mul(&Number::rational(parse_factor("9/5"), prec))),
    }
}

/// Integer value if the symbol text is a number-base unit ("2", "8", …).
pub fn base_unit_value(u: &UnitDef) -> Option<u32> {
    if u.category == "base" {
        u.symbols.first().and_then(|s| s.parse().ok())
    } else {
        None
    }
}

pub fn int_of(i: Integer) -> i64 {
    i.to_i64().unwrap_or(0)
}
