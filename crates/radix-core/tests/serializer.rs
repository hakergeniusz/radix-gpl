//! Serializer compatibility tests (spec: doc 09 §7 FMT-T001..017 plus
//! edge cases). All expected strings are byte-exact from the catalog.

use radix_core::number::parse_decimal_rational;
use radix_core::serializer::{NumberFormat, Serializer, NARROW_SPACE};
use radix_core::Number;
use rug::{Float, Integer, Rational};

fn ser(fmt: NumberFormat) -> Serializer {
    Serializer {
        accuracy: 9,
        show_thousands: true,
        show_zeroes: false,
        format: fmt,
        radix: '.',
        thousands: ',',
        repr_base: 10,
        parser_base: 10,
        leading_digits: 12,
    }
}

/// Integer value parsed in `base`, displayed in `base` (no suffix).
fn in_base(s: &str, base: u32, fmt: NumberFormat) -> String {
    let v = Number::integer(Integer::from_str_radix(s, base as i32).unwrap(), 2000);
    Serializer {
        repr_base: base,
        parser_base: base,
        ..ser(fmt)
    }
    .serialize(&v)
    .unwrap()
}

fn dec(s: &str) -> Number {
    Number::rational(parse_decimal_rational(s).unwrap(), 2000)
}

fn int10(v: i64) -> Number {
    Number::int(v, 2000)
}

fn complex(re: f64, im: f64) -> Number {
    Number::complex(Float::with_val(2000, re), Float::with_val(2000, im))
}

// ---------------------------------------------------------------------
// FMT-T001..T005 — fixed format
// ---------------------------------------------------------------------

#[test]
fn fmt_t001_fixed_grouping_decimal() {
    let v = dec("123456789012345678901234");
    assert_eq!(
        ser(NumberFormat::Fixed).serialize(&v).unwrap(),
        "123,456,789,012,345,678,901,234"
    );
}

#[test]
fn fmt_t002_fixed_fraction_rounding() {
    let v = dec("0.1234567890123456789012");
    assert_eq!(
        ser(NumberFormat::Fixed).serialize(&v).unwrap(),
        "0.123456789"
    );
}

#[test]
fn fmt_t003_fixed_grouping_binary() {
    assert_eq!(
        in_base("101010101010101010101010", 2, NumberFormat::Fixed),
        ["1010", "1010", "1010", "1010", "1010", "1010"].join(&NARROW_SPACE.to_string())
    );
}

#[test]
fn fmt_t004_fixed_grouping_octal() {
    assert_eq!(
        in_base("123456701234567012345670", 8, NumberFormat::Fixed),
        ["1234", "5670", "1234", "5670", "1234", "5670"].join(&NARROW_SPACE.to_string())
    );
}

#[test]
fn fmt_t005_fixed_grouping_hex() {
    assert_eq!(
        in_base("123456789ABCDEF012345678", 16, NumberFormat::Fixed),
        ["1234", "5678", "9ABC", "DEF0", "1234", "5678"].join(&NARROW_SPACE.to_string())
    );
}

// ---------------------------------------------------------------------
// FMT-T006..T009 — automatic format and thresholds
// ---------------------------------------------------------------------

#[test]
fn fmt_t006_automatic_decimal_to_scientific() {
    let v = dec("12345678901234567890");
    assert_eq!(
        ser(NumberFormat::Automatic).serialize(&v).unwrap(),
        "1.23456789×10¹⁹"
    );
}

#[test]
fn fmt_t007_automatic_rounding_cascades_into_exponent() {
    let v = dec("9999999999500");
    assert_eq!(
        ser(NumberFormat::Automatic).serialize(&v).unwrap(),
        "1×10¹³"
    );
}

#[test]
fn fmt_t008_automatic_binary_threshold() {
    // ≤ 64 digits stays plain (grouped in 4s): all 64 bit patterns.
    let v = Number::integer((Integer::from(1) << 64u32) - 1, 2000);
    let s = in_base_helper(&v, 2, NumberFormat::Automatic);
    let grouped: String = std::iter::repeat_n("1111", 16)
        .collect::<Vec<_>>()
        .join(&NARROW_SPACE.to_string());
    assert_eq!(s, grouped);
    // A 65-digit binary number overflows: exponent 64 rendered in binary.
    let v65 = Number::integer(
        Integer::from_str_radix(
            "10101010101010101010101010101010101010101010101010101010101010101",
            2,
        )
        .unwrap(),
        2000,
    );
    let s = in_base_helper(&v65, 2, NumberFormat::Automatic);
    assert_eq!(s, "1.010101011×10¹⁰⁰⁰⁰⁰⁰");
}

#[test]
fn fmt_t009_automatic_hex_exponent_in_base() {
    let v = Number::integer(
        Integer::from_str_radix("123456789ABCEDF01", 16).unwrap(),
        2000,
    );
    let s = in_base_helper(&v, 16, NumberFormat::Automatic);
    assert_eq!(s, "1.23456789B×10¹⁰");
}

fn in_base_helper(v: &Number, base: u32, fmt: NumberFormat) -> String {
    Serializer {
        repr_base: base,
        parser_base: base,
        ..ser(fmt)
    }
    .serialize(v)
    .unwrap()
}

// ---------------------------------------------------------------------
// FMT-T010..T012 — scientific and engineering
// ---------------------------------------------------------------------

#[test]
fn fmt_t010_scientific() {
    let s = ser(NumberFormat::Scientific);
    assert_eq!(s.serialize(&int10(10)).unwrap(), "1×10¹");
    assert_eq!(s.serialize(&dec("0.1")).unwrap(), "1×10⁻¹");
    assert_eq!(s.serialize(&dec("1234567890")).unwrap(), "1.23456789×10⁹");
    // No exponent for 10^0.
    assert_eq!(s.serialize(&int10(7)).unwrap(), "7");
}

#[test]
fn fmt_t011_scientific_non_decimal_is_fixed() {
    // Doc 07 §2.3: explicit scientific in a non-decimal base falls back to
    // fixed with grouping (decision D17).
    assert_eq!(
        in_base("101010101010101010101010", 2, NumberFormat::Scientific),
        ["1010", "1010", "1010", "1010", "1010", "1010"].join(&NARROW_SPACE.to_string())
    );
}

#[test]
fn fmt_t012_engineering() {
    let s = ser(NumberFormat::Engineering);
    assert_eq!(s.serialize(&dec("0.1")).unwrap(), "100×10⁻³");
    assert_eq!(
        s.serialize(&dec("0.1234567890")).unwrap(),
        "123.456789×10⁻³"
    );
    assert_eq!(s.serialize(&dec("999999999999500")).unwrap(), "1×10¹⁵");
}

// ---------------------------------------------------------------------
// FMT-T013..T015 — base suffixes and trailing zeroes
// ---------------------------------------------------------------------

#[test]
fn fmt_t013_base_suffix() {
    let v = int10(10);
    for (base, want) in [(2u32, "1010₂"), (8, "12₈"), (16, "A₁₆")] {
        let s = Serializer {
            repr_base: base,
            ..ser(NumberFormat::Automatic)
        };
        assert_eq!(s.serialize(&v).unwrap(), want, "base {base}");
    }
}

#[test]
fn fmt_t014_base_suffix_parser_base_mismatch() {
    // `12₈` re-rendered with representation base 2.
    let v = Number::integer(Integer::from_str_radix("12", 8).unwrap(), 2000);
    let s = Serializer {
        repr_base: 2,
        parser_base: 8,
        ..ser(NumberFormat::Automatic)
    };
    assert_eq!(s.serialize(&v).unwrap(), "1010₂");
    // Same parser and representation base: no suffix.
    let s = Serializer {
        repr_base: 8,
        parser_base: 8,
        ..ser(NumberFormat::Automatic)
    };
    assert_eq!(s.serialize(&v).unwrap(), "12");
}

#[test]
fn fmt_t015_show_zeroes() {
    let v = dec("0.10");
    let on = Serializer {
        show_zeroes: true,
        ..ser(NumberFormat::Fixed)
    };
    assert_eq!(on.serialize(&v).unwrap(), "0.100000000");
    let off = ser(NumberFormat::Fixed);
    assert_eq!(off.serialize(&v).unwrap(), "0.1");
}

// ---------------------------------------------------------------------
// FMT-T016..T017 — complex and special values
// ---------------------------------------------------------------------

#[test]
fn fmt_t016_complex() {
    let s = ser(NumberFormat::Fixed);
    assert_eq!(s.serialize(&complex(1.0, 1.0)).unwrap(), "1+i");
    assert_eq!(s.serialize(&complex(-1.0, 1.0)).unwrap(), "−1+i");
    assert_eq!(s.serialize(&complex(0.0, -1.0)).unwrap(), "−i");
    assert_eq!(s.serialize(&complex(0.0, 2.0)).unwrap(), "2i");
    assert_eq!(s.serialize(&complex(1.0, -1.0)).unwrap(), "1−i");
    // Zero imaginary part displays as a real number.
    assert_eq!(s.serialize(&complex(3.0, 0.0)).unwrap(), "3");
    // Both parts rounding to zero show the zero string.
    assert_eq!(s.serialize(&complex(1e-30, 1e-30)).unwrap(), "0");
}

#[test]
fn fmt_t017_special_values() {
    let s = ser(NumberFormat::Fixed);
    assert_eq!(
        s.serialize(&Number::from_f64(f64::INFINITY, 2000)).unwrap(),
        "∞"
    );
    assert_eq!(
        s.serialize(&Number::from_f64(f64::NEG_INFINITY, 2000))
            .unwrap(),
        "−∞"
    );
    assert_eq!(
        s.serialize(&Number::from_f64(f64::NAN, 2000)).unwrap(),
        "NaN"
    );
}

// ---------------------------------------------------------------------
// Rounding, carry and cap edge cases
// ---------------------------------------------------------------------

#[test]
fn fixed_rounds_half_away_from_zero() {
    let s = ser(NumberFormat::Fixed);
    // Exactly half at digit A+1 rounds away from zero.
    assert_eq!(s.serialize(&dec("0.0000000005")).unwrap(), "0.000000001");
    assert_eq!(s.serialize(&dec("0.1234567895")).unwrap(), "0.12345679");
    assert_eq!(s.serialize(&dec("-0.1234567895")).unwrap(), "−0.12345679");
    // Carry through the radix point.
    assert_eq!(s.serialize(&dec("999.9999999999")).unwrap(), "1,000");
    // A value that rounds to zero loses its sign.
    assert_eq!(s.serialize(&dec("-0.0000000001")).unwrap(), "0");
}

#[test]
fn fixed_cap_and_scientific_exemption() {
    // 2^2000 equals the cap: fixed and automatic error, scientific renders.
    let v = Number::integer(Integer::from(1) << 2000u32, 2000);
    let err = ser(NumberFormat::Fixed).serialize(&v).unwrap_err();
    assert_eq!(
        err.message,
        "The result is too long to display in fixed format. Try other result formats"
    );
    assert!(ser(NumberFormat::Automatic).serialize(&v).is_err());
    assert_eq!(
        ser(NumberFormat::Scientific).serialize(&v).unwrap(),
        "1.148130695×10⁶⁰²"
    );
    // Just under the cap renders in fixed.
    let v = Number::integer((Integer::from(1) << 2000u32) - 1, 2000);
    assert!(ser(NumberFormat::Fixed).serialize(&v).is_ok());
}

#[test]
fn automatic_large_binary_integer() {
    // 2^100 in decimal: 31 digits > 12 → scientific.
    let v = Number::integer(Integer::from(1) << 100u32, 2000);
    assert_eq!(
        ser(NumberFormat::Automatic).serialize(&v).unwrap(),
        "1.2676506×10³⁰"
    );
}

#[test]
fn zero_and_negative_zero() {
    let s = ser(NumberFormat::Automatic);
    assert_eq!(s.serialize(&int10(0)).unwrap(), "0");
    let zeros = Serializer {
        show_zeroes: true,
        ..ser(NumberFormat::Fixed)
    };
    assert_eq!(zeros.serialize(&int10(0)).unwrap(), "0.000000000");
}

#[test]
fn negative_fixed_grouping() {
    let v = dec("-123456789012345678901234");
    assert_eq!(
        ser(NumberFormat::Fixed).serialize(&v).unwrap(),
        "−123,456,789,012,345,678,901,234"
    );
}

#[test]
fn exact_fraction_keeps_digits() {
    // 1/3 rounds at A=9 with no binary residue.
    let v = Number::rational(Rational::from(1) / Rational::from(3), 2000);
    assert_eq!(
        ser(NumberFormat::Fixed).serialize(&v).unwrap(),
        "0.333333333"
    );
    // 0.1 + 0.2 − 0.3 is exactly zero through the rational shadow.
    let v = dec("0.1").add(&dec("0.2")).sub(&dec("0.3"));
    assert_eq!(ser(NumberFormat::Fixed).serialize(&v).unwrap(), "0");
}

#[test]
fn scientific_small_values() {
    let s = ser(NumberFormat::Scientific);
    // 2^-2000 = 8.7098098162172…×10⁻⁶⁰³ (computed independently).
    let v = Number::integer(Integer::from(1), 2000)
        .div(&Number::integer(Integer::from(1) << 2000u32, 2000))
        .unwrap();
    assert_eq!(s.serialize(&v).unwrap(), "8.709809816×10⁻⁶⁰³");
    assert_eq!(s.serialize(&dec("0.1")).unwrap(), "1×10⁻¹");
}

#[test]
fn dms_format() {
    let s = ser(NumberFormat::Fixed);
    assert_eq!(s.serialize_dms(&dec("10.5")), "10°30′0″");
    assert_eq!(s.serialize_dms(&dec("-10.5")), "−10°30′0″");
    // 1° + 1′ + 1″.
    let v = dec("1")
        .add(&dec("1").div(&dec("60")).unwrap())
        .add(&dec("1").div(&dec("3600")).unwrap());
    assert_eq!(s.serialize_dms(&v), "1°1′1″");
    // 59.9999999997″ snaps to the next minute via the carry.
    let v = dec("1").add(&dec("59.9999999997").div(&dec("3600")).unwrap());
    assert_eq!(s.serialize_dms(&v), "1°1′0″");
    assert_eq!(s.serialize_dms(&int10(0)), "0°0′0″");
}

#[test]
fn register_strings() {
    assert_eq!(
        Serializer::register_string(&dec("12345678901234567890")),
        "1.234567890123456789×10¹⁹"
    );
    assert_eq!(Serializer::register_string(&int10(5)), "5");
    // Registers store the scientific format (doc 08 §2): 0.5 gains an exponent.
    assert_eq!(Serializer::register_string(&dec("0.5")), "5×10⁻¹");
    assert_eq!(Serializer::register_string(&dec("-2.75")), "−2.75");
}

#[test]
fn localized_radix_and_thousands() {
    // LOC-T002/003 serializer side (doc 09 §10): the radix and thousands
    // characters are the locale's, e.g. de_DE (`.` groups, `,` radix).
    let de = Serializer {
        radix: ',',
        thousands: '.',
        ..ser(NumberFormat::Fixed)
    };
    assert_eq!(de.serialize(&dec("1234.5")).unwrap(), "1.234,5");
    // Grouping is suppressed when the separator equals the radix (doc 07 §1).
    let degenerate = Serializer {
        radix: ',',
        thousands: ',',
        ..ser(NumberFormat::Fixed)
    };
    assert_eq!(degenerate.serialize(&dec("1234.5")).unwrap(), "1234,5");
    // C locale unaffected.
    assert_eq!(
        ser(NumberFormat::Fixed).serialize(&dec("1234.5")).unwrap(),
        "1,234.5"
    );
}
