//! End-to-end compatibility tests against the doc 09 catalog, sections 1–6
//! (expressions, literals, numerics, functions, variables, units).
//!
//! The harness solves canonical expressions and serializes with the engine
//! settings the catalog's expected strings use: fixed format, no grouping,
//! accuracy 9 (display-format cases with grouping live in serializer.rs).

use radix_core::currency::CurrencyTable;
use radix_core::error::ErrorCode;
use radix_core::eval::{EvalConfig, EvalState};
use radix_core::numfuncs::AngleUnit;
use radix_core::serializer::{NumberFormat, Serializer};
use radix_core::solve::solve;
use radix_core::units::UnitCatalog;

struct App {
    state: EvalState,
    units: UnitCatalog,
    currencies: CurrencyTable,
    config: EvalConfig,
    ser: Serializer,
}

impl App {
    fn new() -> Self {
        Self::with(10, AngleUnit::Degrees, 64)
    }

    fn with(base: u32, angle: AngleUnit, wordlen: u32) -> Self {
        App {
            state: EvalState::default(),
            units: UnitCatalog::new(),
            currencies: CurrencyTable::new(),
            config: EvalConfig {
                precision: 2000,
                wordlen,
                angle_unit: angle,
                base,
            },
            ser: Serializer {
                format: NumberFormat::Fixed,
                show_thousands: false,
                ..Serializer::default()
            },
        }
    }

    fn base(base: u32) -> Self {
        Self::with(base, AngleUnit::Degrees, 64)
    }

    fn hex32() -> Self {
        Self::with(16, AngleUnit::Degrees, 32)
    }

    fn display_grouping(mut self) -> Self {
        self.ser.show_thousands = true;
        self
    }

    fn solve(&mut self, input: &str) -> Result<String, radix_core::Error> {
        let out = solve(
            input,
            &self.config,
            &mut self.state,
            &self.units,
            &self.currencies,
            None,
        );
        if let Some(e) = out.error {
            return Err(e);
        }
        let v = out.value.expect("solved value present");
        self.ser.parser_base = self.config.base;
        self.ser.repr_base = if out.repr_base == 0 {
            self.config.base
        } else {
            out.repr_base
        };
        self.ser.serialize(&v)
    }

    fn err(&mut self, input: &str) -> radix_core::Error {
        self.solve(input).unwrap_err()
    }
}

// =====================================================================
// 1. Expression parsing (EXPR-T001..T041)
// =====================================================================

#[test]
fn expr_t001_t002_precedence_and_parens() {
    let mut a = App::new();
    assert_eq!(a.solve("1+2×3").unwrap(), "7");
    assert_eq!(a.solve("(1+2)×3").unwrap(), "9");
}

#[test]
fn expr_t003_t004_t005_implicit_multiply() {
    let mut a = App::new();
    assert_eq!(a.solve("4÷2(1+1)").unwrap(), "4");
    assert_eq!(a.solve("(2)1+1").unwrap(), "3");
    assert_eq!(a.solve("(2)(1)").unwrap(), "2");
}

#[test]
fn expr_t006_t007_t008_unary_minus_and_power() {
    let mut a = App::new();
    assert_eq!(a.solve("−10^2").unwrap(), "−100");
    assert_eq!(a.solve("(−10)^2").unwrap(), "100");
    assert_eq!(a.solve("4^3^2").unwrap(), "262144");
}

#[test]
fn expr_t009_t010_mod_precedence() {
    let mut a = App::new();
    assert_eq!(a.solve("10 mod 4 / 2").unwrap(), "1");
    assert_eq!(a.solve("20 / 10 mod 3").unwrap(), "2");
}

#[test]
fn expr_t011_t012_t013_t014_roots_factorial() {
    let mut a = App::new();
    assert_eq!(a.solve("12 / 3 √4").unwrap(), "8");
    assert_eq!(a.solve("√5!").unwrap(), "10.95445115");
    assert_eq!(a.solve("6 + 3!").unwrap(), "12");
    assert_eq!(a.solve("3! ^ 3").unwrap(), "216");
    assert_eq!(a.solve("3 ^ 3!").unwrap(), "729");
}

#[test]
fn expr_t015_t019_t021_t022_t035_percentage() {
    let mut a = App::new();
    assert_eq!(a.solve("100+20%").unwrap(), "120");
    assert_eq!(a.solve("100−20%").unwrap(), "80");
    assert_eq!(a.solve("100 - 20% * 100").unwrap(), "80");
    assert_eq!(a.solve("100 + 20% * 100").unwrap(), "120");
    assert_eq!(a.solve("100 - 100 * 20%").unwrap(), "80");
    assert_eq!(a.solve("100%").unwrap(), "1");
    assert_eq!(a.solve("1%").unwrap(), "0.01");
    assert_eq!(a.solve("(((100−20)−50%)+20)").unwrap(), "60");
    assert_eq!(a.solve("(50+50)+10%").unwrap(), "110");
    assert_eq!(a.solve("(50+50)-10%").unwrap(), "90");
}

#[test]
fn expr_t020_trailing_equals_tolerated() {
    let mut a = App::new();
    assert_eq!(a.solve("(50+50)+10%=").unwrap(), "110");
}

#[test]
fn expr_t023_t025_t026_t027_t028_superscripts_and_roots() {
    let mut a = App::new();
    assert_eq!(a.solve("2²").unwrap(), "4");
    assert_eq!(a.solve("2¹⁰").unwrap(), "1024");
    assert_eq!(a.solve("2⁻¹").unwrap(), "0.5");
    assert_eq!(a.solve("√4").unwrap(), "2");
    assert_eq!(a.solve("∛8").unwrap(), "2");
    assert_eq!(a.solve("∜16").unwrap(), "2");
    assert_eq!(a.solve("₃√8").unwrap(), "2");
    assert_eq!(a.solve("₁₀√1024").unwrap(), "2");
    assert_eq!(a.solve("2√4").unwrap(), "4");
    a.solve("x=2").unwrap();
    assert_eq!(a.solve("x√4").unwrap(), "4");
}

#[test]
fn expr_t029_t030_zero_power() {
    let mut a = App::new();
    let e = a.err("0^0");
    assert_eq!(e.code, ErrorCode::Mp);
    assert_eq!(e.message, "Zero raised to zero is undefined");
    assert_eq!(a.solve("0^0.5").unwrap(), "0");
}

#[test]
fn expr_t031_mixed_precedence() {
    let mut a = App::new();
    assert_eq!(a.solve("1+2 - 3 * 4 / 5").unwrap(), "0.6");
}

#[test]
fn expr_t032_t033_t034_unary_plus() {
    let mut a = App::new();
    assert_eq!(a.solve("+1").unwrap(), "1");
    assert_eq!(a.solve("+ 1").unwrap(), "1");
    assert_eq!(a.err("++1").code, ErrorCode::Invalid);
    assert_eq!(a.solve("−−1").unwrap(), "1");
}

#[test]
fn expr_t036_exp_notation() {
    let mut a = App::new();
    assert_eq!(a.solve("2×10^3").unwrap(), "2000");
    assert_eq!(a.solve("2×10^−3").unwrap(), "0.002");
}

#[test]
fn expr_t037_cdot() {
    let mut a = App::new();
    assert_eq!(a.solve("2\\cdot3").unwrap(), "6");
}

#[test]
fn expr_t038_t039_t040_t041_invalids() {
    let mut a = App::new();
    for bad in ["", "+", ".", "−", "1+", "a=b=c", "1 2"] {
        assert_eq!(a.err(bad).code, ErrorCode::Invalid, "input {bad:?}");
    }
}

#[test]
fn expr_t024_missing_sup_digits() {
    let mut a = App::new();
    let e = a.err("2⁻");
    assert_eq!(e.code, ErrorCode::Mp);
    assert_eq!(e.message, "Missing digits after minus sign");
}

// =====================================================================
// 2. Literals and bases (LIT-T001..T018)
// =====================================================================

#[test]
fn lit_t001_base_suffixes_and_prefixes() {
    let mut a = App::base(10);
    assert_eq!(a.solve("1010₂").unwrap(), "10");
    assert_eq!(a.solve("A₁₆").unwrap(), "10");
    let mut h = App::base(16);
    assert_eq!(h.solve("FF").unwrap(), "FF");
}

#[test]
fn lit_t002_digit_above_suffix_base() {
    let mut a = App::base(10);
    assert_eq!(a.err("2₂").code, ErrorCode::Invalid);
}

#[test]
fn lit_t003_prefixes_any_base() {
    let mut a = App::base(10);
    assert_eq!(a.solve("0xA").unwrap(), "10");
    assert_eq!(a.solve("0b101").unwrap(), "5");
    assert_eq!(a.solve("0o17").unwrap(), "15");
    let mut h = App::base(16);
    assert_eq!(h.solve("0xA").unwrap(), "A");
}

#[test]
fn lit_t004_t018_prefix_digit_errors() {
    let mut a = App::base(10);
    let e = a.err("0b2");
    assert_eq!(e.code, ErrorCode::Mp);
    assert!(a.solve("0b10103").is_err());
}

#[test]
fn lit_t005_t006_prefix_suffix_agreement() {
    let mut a = App::base(10);
    assert_eq!(a.solve("0b1₂").unwrap(), "1");
    assert_eq!(a.err("0x1₂").code, ErrorCode::Invalid);
}

#[test]
fn lit_t007_decimal_literal_in_binary_base() {
    let mut b = App::base(2);
    assert_eq!(b.solve("2₁₀").unwrap(), "10");
}

#[test]
fn lit_t008_big_literal_via_suffix() {
    let mut h = App::base(16);
    assert_eq!(
        h.solve("18364758544493064720₁₀").unwrap(),
        "FEDCBA9876543210"
    );
}

#[test]
fn lit_t009_vulgar_fractions() {
    let mut a = App::new();
    assert_eq!(a.solve("½").unwrap(), "0.5");
    assert_eq!(a.solve("1½").unwrap(), "1.5");
}

#[test]
fn lit_t010_t011_t012_t015_dms_literals() {
    let mut a = App::new();
    assert_eq!(a.solve("0°30'").unwrap(), "0.5");
    assert_eq!(a.solve("0°0'1\"").unwrap(), "0.000277778");
    assert_eq!(a.solve("0°0'0.1\"").unwrap(), "0.000027778");
    assert_eq!(a.solve("0°").unwrap(), "0");
}

#[test]
fn lit_t013_t014_unicode_digits() {
    let mut a = App::new();
    assert_eq!(a.solve("١٢٣٤٥٦٧٨٩٠").unwrap(), "1234567890");
    assert_eq!(a.solve("۱۲۳۴۵۶۷۸۹۰").unwrap(), "1234567890");
}

#[test]
fn lit_t016_t017_fractions_in_base() {
    let mut b = App::base(2);
    assert_eq!(b.solve("0.10101010101010101010").unwrap(), "0.101010101");
    let mut a = App::new();
    assert_eq!(a.solve("1.01").unwrap(), "1.01");
}

// =====================================================================
// 3. Arithmetic and numeric semantics (NUM-T001..T030)
// =====================================================================

#[test]
fn num_t001_t005_exact_rationals() {
    let mut a = App::new();
    assert_eq!(a.solve("0.1+0.2").unwrap(), "0.3");
    assert_eq!(a.solve("1÷3×3").unwrap(), "1");
    assert_eq!(a.solve("2÷3+1÷3").unwrap(), "1");
    assert_eq!(a.solve("44.23−8.10−11.23−6.9−18").unwrap(), "0");
    // 19-digit fractions stay exact (three addends per the upstream test).
    assert_eq!(
        a.solve("0.3333333333333333333+0.3333333333333333333+0.3333333333333333333−0.9999999999999999999")
            .unwrap(),
        "0"
    );
}

#[test]
fn num_t006_division_by_zero() {
    let mut a = App::new();
    for bad in ["1÷0", "0÷0", "0.0/0.0"] {
        let e = a.err(bad);
        assert_eq!(e.code, ErrorCode::Mp, "{bad}");
        assert_eq!(e.message, "Division by zero is undefined");
    }
}

#[test]
fn num_t007_t008_big_exact_values() {
    let mut a = App::new();
    assert_eq!(a.solve("2^100").unwrap(), "1267650600228229401496703205376");
    assert_eq!(a.solve("0!").unwrap(), "1");
    assert_eq!(a.solve("5!").unwrap(), "120");
    assert_eq!(
        a.solve("69!").unwrap(),
        "171122452428141311372468338881272839092270544893520369393648040923257279754140647424000000000000000"
    );
}

#[test]
fn num_t009_t010_t011_gamma_factorial() {
    let mut a = App::new();
    assert_eq!(a.solve("3.5!").unwrap(), "11.631728397");
    assert_eq!(a.solve("(−10.9)!").unwrap(), "0.000003543");
    assert_eq!(a.err("(−1)!").code, ErrorCode::Mp);
    assert_eq!(a.err("(−10.0)!").code, ErrorCode::Mp);
    assert_eq!(a.solve("−1!").unwrap(), "−1");
}

#[test]
fn num_t012_mod_signs() {
    let mut a = App::new();
    assert_eq!(a.solve("−1 mod 7").unwrap(), "6");
    assert_eq!(a.solve("21 mod -9").unwrap(), "−6");
    assert_eq!(a.solve("-21 mod 9").unwrap(), "6");
    assert_eq!(a.solve("-21 mod -9").unwrap(), "−3");
}

#[test]
fn num_t013_t014_t015_rounding_family() {
    let mut a = App::new();
    assert_eq!(a.solve("⌊3.5⌋").unwrap(), "3");
    assert_eq!(a.solve("[3.5]").unwrap(), "4");
    assert_eq!(a.solve("⌈3.5⌉").unwrap(), "4");
    assert_eq!(a.solve("[−3.5]").unwrap(), "−4");
    assert_eq!(a.solve("{−3.2}").unwrap(), "0.8");
    assert_eq!(a.solve("frac(−3.2)").unwrap(), "−0.2");
}

#[test]
fn num_t016_t017_t018_abs_sgn_negzero() {
    let mut a = App::new();
    assert_eq!(a.solve("|3−5|").unwrap(), "2");
    assert_eq!(a.solve("|i|").unwrap(), "1");
    assert_eq!(a.solve("|3+4i|").unwrap(), "5");
    assert_eq!(a.solve("sgn −3").unwrap(), "−1");
    assert_eq!(a.solve("sgn 0").unwrap(), "0");
    assert_eq!(a.solve("−0÷32352.689").unwrap(), "0");
}

#[test]
fn num_t019_t020_t021_combinatorics() {
    let mut a = App::new();
    assert_eq!(a.solve("ncr (9;5)").unwrap(), "126");
    assert_eq!(a.solve("npr (9;5)").unwrap(), "15120");
    assert_eq!(a.err("ncr (9;10)").code, ErrorCode::Mp);
    assert_eq!(a.solve("gcd (456;1584)").unwrap(), "24");
    assert_eq!(a.solve("gcd (456;1584;40;60)").unwrap(), "4");
    assert_eq!(a.solve("lcm (456;1584)").unwrap(), "30096");
    assert_eq!(a.err("gcd (456;15.84)").code, ErrorCode::Mp);
}

#[test]
fn num_t022_t023_precision_and_cap() {
    let mut a = App::new();
    assert_eq!(a.solve("1000000000000000×0.000000000000001").unwrap(), "1");
    // ≥ 2^2000 in fixed format → serializer error, not a value.
    let e = a.err("2^2000");
    assert_eq!(
        e.message,
        "The result is too long to display in fixed format. Try other result formats"
    );
}

#[test]
fn num_t024_t025_t026_bitwise_hex32() {
    let mut a = App::hex32();
    assert_eq!(a.solve("twos 1").unwrap(), "FFFFFFFF");
    assert_eq!(a.solve("twos 7FFFFFFF").unwrap(), "80000001");
    assert_eq!(a.solve("not 1").unwrap(), "FFFFFFFE");
    assert_eq!(a.solve("~7A₁₆").unwrap(), "FFFFFF85");
    let mut b = App::with(2, AngleUnit::Degrees, 4);
    assert_eq!(b.solve("1100∧1010").unwrap(), "1000");
    assert_eq!(b.solve("1100⊼1010").unwrap(), "111");
    assert_eq!(b.solve("1100⊽1010").unwrap(), "1");
    assert_eq!(b.solve("1100⊙1010").unwrap(), "1001");
}

#[test]
fn num_t027_t028_shifts() {
    let mut a = App::with(10, AngleUnit::Degrees, 64);
    assert_eq!(a.solve("1«2").unwrap(), "4");
    assert_eq!(a.solve("1«2»1«2").unwrap(), "8");
    assert_eq!(a.solve("1«35").unwrap(), "34359738368");
    assert_eq!(a.solve("1«63").unwrap(), "9223372036854775808");
    let mut w8 = App::with(10, AngleUnit::Degrees, 8);
    let e = w8.err("255«1");
    assert_eq!(e.message, "Overflow. Try a bigger word size");
}

#[test]
fn num_t029_modexp() {
    let mut a = App::new();
    assert_eq!(a.solve("modexp(7;11;13)").unwrap(), "2");
}

#[test]
fn num_t030_rand() {
    let mut a = App::new();
    let r1 = a.solve("rand").unwrap();
    let r2 = a.solve("rand").unwrap();
    for r in [&r1, &r2] {
        let v: f64 = r.parse().unwrap();
        assert!((0.0..1.0).contains(&v), "{r}");
    }
    assert_ne!(r1, r2);
}

// =====================================================================
// 4. Functions and constants (FUNC-T001..T025)
// =====================================================================

#[test]
fn func_t001_t002_t003_trig_axes() {
    let mut a = App::new();
    assert_eq!(a.solve("sin 90").unwrap(), "1");
    assert_eq!(a.solve("sin 180").unwrap(), "0");
    assert_eq!(a.solve("cos 90").unwrap(), "0");
    assert_eq!(a.solve("cos 180").unwrap(), "−1");
    assert_eq!(a.err("tan 90").code, ErrorCode::Mp);
}

#[test]
fn func_t004_t005_trig_values() {
    let mut a = App::new();
    assert_eq!(a.solve("sin 45 − 1÷√2").unwrap(), "0");
    assert_eq!(a.solve("sin²45").unwrap(), "0.5");
    assert_eq!(a.solve("cos²45").unwrap(), "0.5");
    assert_eq!(a.solve("tan²10").unwrap(), "0.031091204");
}

#[test]
fn func_t006_angle_units() {
    let mut r = App::with(10, AngleUnit::Radians, 64);
    assert_eq!(r.solve("sin (π÷2)").unwrap(), "1");
    let mut g = App::with(10, AngleUnit::Gradians, 64);
    assert_eq!(g.solve("sin 100").unwrap(), "1");
}

#[test]
fn func_t007_t008_inverse_trig() {
    let mut a = App::new();
    assert_eq!(a.solve("cos⁻¹ 0").unwrap(), "90");
    assert_eq!(a.solve("sin⁻¹ 1").unwrap(), "90");
    assert_eq!(a.solve("sin⁻¹ (−1)").unwrap(), "−90");
    assert_eq!(a.solve("asin 1").unwrap(), "90");
}

#[test]
fn func_t009_mode_dependence() {
    let mut a = App::new();
    assert_eq!(a.solve("sin 2").unwrap(), "0.034899497");
}

#[test]
fn func_t010_t011_hyperbolic_identities() {
    let mut a = App::new();
    assert_eq!(a.solve("atanh (1÷10) − 0.5 ln(11÷9)").unwrap(), "0");
    assert_eq!(a.solve("cosh 10 − (e^10 + e^−10)÷2").unwrap(), "0");
    assert_eq!(a.solve("sinh (−10) + sinh 10").unwrap(), "0");
}

#[test]
fn func_t012_t013_t014_logs() {
    let mut a = App::new();
    assert_eq!(a.solve("log 10").unwrap(), "1");
    assert_eq!(a.solve("log 2").unwrap(), "0.301029996");
    assert_eq!(a.solve("log₁₀ 10").unwrap(), "1");
    assert_eq!(a.solve("log₂ 2").unwrap(), "1");
    assert_eq!(a.solve("ln 1").unwrap(), "0");
    assert_eq!(a.solve("ln 2").unwrap(), "0.693147181");
    assert_eq!(a.solve("ln e").unwrap(), "1");
    assert_eq!(a.err("log 0").code, ErrorCode::Mp);
    assert_eq!(a.err("ln 0").code, ErrorCode::Mp);
}

#[test]
fn func_t015_t016_sqrt_exp() {
    let mut a = App::new();
    assert_eq!(a.solve("Sqrt (2)").unwrap(), "1.414213562");
    assert_eq!(a.solve("Sqrt (-1)").unwrap(), "i");
    assert_eq!(a.solve("e^(−0.5)").unwrap(), "0.60653066");
}

#[test]
fn func_t017_constants() {
    let mut a = App::new();
    assert_eq!(a.solve("π").unwrap(), "3.141592654");
    assert_eq!(a.solve("e").unwrap(), "2.718281828");
}

#[test]
fn func_t018_t019_t020_complex() {
    let mut a = App::new();
    assert_eq!(a.solve("i").unwrap(), "i");
    assert_eq!(a.solve("i×i").unwrap(), "−1");
    assert_eq!(a.solve("1÷i").unwrap(), "−i");
    assert_eq!(a.solve("i^2").unwrap(), "−1");
    assert_eq!(a.solve("1+i").unwrap(), "1+i");
    assert_eq!(a.solve("i−1").unwrap(), "−1+i");
    assert_eq!(a.solve("2i").unwrap(), "2i");
    assert_eq!(a.solve("−i").unwrap(), "−i");
    assert_eq!(a.solve("(−1)^(1/2)").unwrap(), "i");
    assert_eq!(a.solve("√−4").unwrap(), "2i");
    assert_eq!(a.solve("e^iπ+1").unwrap(), "0");
}

#[test]
fn func_t021_arg() {
    let mut a = App::new();
    assert_eq!(a.solve("arg (1+i)").unwrap(), "45");
    assert_eq!(a.solve("arg −1").unwrap(), "180");
    assert_eq!(a.solve("arg −i").unwrap(), "−90");
    assert_eq!(a.err("arg 0").code, ErrorCode::Mp);
}

#[test]
fn func_t022_t023_complex_log_and_trig() {
    let mut a = App::new();
    assert_eq!(a.solve("log (−10) − (1 + πi÷ln(10))").unwrap(), "0");
    assert_eq!(a.solve("ln (−e) − (1 + πi)").unwrap(), "0");
    assert_eq!(a.solve("sin(iπ÷4) − i×sinh(π÷4)").unwrap(), "0");
    assert_eq!(a.solve("cos(iπ÷4) − cosh(π÷4)").unwrap(), "0");
}

#[test]
fn func_t024_t025_abs_and_unknown() {
    let mut a = App::new();
    assert_eq!(a.solve("abs (−1)").unwrap(), "1");
    assert_eq!(a.solve("abs 1").unwrap(), "1");
    assert_eq!(a.err("add(8)").code, ErrorCode::UnknownFunction);
    assert_eq!(a.err("add").code, ErrorCode::UnknownVariable);
}

// =====================================================================
// 5. Variables and user functions (VAR-T001..T016)
// =====================================================================

#[test]
fn var_t001_t002_var_products() {
    let mut a = App::new();
    a.solve("x=2").unwrap();
    a.solve("y=3").unwrap();
    assert_eq!(a.solve("x").unwrap(), "2");
    assert_eq!(a.solve("2y").unwrap(), "6");
    assert_eq!(a.solve("xy").unwrap(), "6");
    assert_eq!(a.solve("yx").unwrap(), "6");
    assert_eq!(a.solve("2xy").unwrap(), "12");
    assert_eq!(a.solve("y²").unwrap(), "9");
    assert_eq!(a.solve("x²y").unwrap(), "12");
    assert_eq!(a.solve("xy²").unwrap(), "18");
    assert_eq!(a.solve("(xy)²").unwrap(), "36");
}

#[test]
fn var_t003_unknowns() {
    let mut a = App::new();
    assert_eq!(a.err("z").code, ErrorCode::UnknownVariable);
    assert_eq!(a.err("z2").code, ErrorCode::UnknownVariable);
    assert_eq!(a.err("z 2").code, ErrorCode::UnknownVariable);
    assert_eq!(a.err("z(2)").code, ErrorCode::UnknownFunction);
}

#[test]
fn var_t004_algebraic_forms() {
    let mut a = App::new();
    a.solve("x=2").unwrap();
    a.solve("y=3").unwrap();
    assert_eq!(a.solve("x³+2x²−5").unwrap(), "11");
    assert_eq!(a.solve("2(x+3y)").unwrap(), "22");
    assert_eq!(a.solve("x(x+3y)").unwrap(), "22");
    assert_eq!(a.solve("(x+3y)(2x-4y)").unwrap(), "−88");
}

#[test]
fn var_t005_assignment() {
    let mut a = App::new();
    assert_eq!(a.solve("z=99").unwrap(), "99");
    assert_eq!(a.solve("z").unwrap(), "99");
}

#[test]
fn var_t006_constants_not_overwritten() {
    let mut a = App::new();
    assert_eq!(a.solve("pi=5").unwrap(), "5");
    assert_eq!(a.solve("pi").unwrap(), "3.141592654");
}

#[test]
fn var_t007_t008_t010_t011_user_functions() {
    let mut a = App::new();
    assert_eq!(a.solve("f(x;y;z)=x+y+z").unwrap(), "0");
    assert_eq!(a.solve("f(2;3;5)").unwrap(), "10");
    assert_eq!(a.solve("f(x;y;z)=x+y-z").unwrap(), "0");
    assert_eq!(a.solve("f(2;3;5)").unwrap(), "0");
    assert_eq!(
        a.solve("funcWithDescription(x;y;z)=x+y+z @ Description")
            .unwrap(),
        "0"
    );
    assert_eq!(a.solve("funcWithDescription(2;3;5)").unwrap(), "10");
    a.solve("test(x;y)=x+y").unwrap();
    assert_eq!(a.solve("test(test(5;6);9)").unwrap(), "20");
}

#[test]
fn var_t009_comma_args_invalid() {
    let mut a = App::new();
    assert_eq!(a.err("diff(x,y)=x-y").code, ErrorCode::Invalid);
}

#[test]
fn var_t012_arity_and_unknown_call() {
    let mut a = App::new();
    a.solve("f(x;y;z)=x+y+z").unwrap();
    assert!(a.solve("f(2;3)").is_err());
    // Defined name with too many arguments also errors.
    assert!(a.solve("f(2;3;5;7)").is_err());
}

#[test]
fn var_t013_builtin_name_reserved() {
    let mut a = App::new();
    assert_eq!(a.err("sin(x)=x").code, ErrorCode::Invalid);
}

#[test]
fn var_t015_answer_reference() {
    let mut a = App::new();
    a.solve("2+3").unwrap();
    assert_eq!(a.solve("_×2").unwrap(), "10");
}

#[test]
fn var_t016_unknown_in_product() {
    let mut a = App::new();
    assert_eq!(a.err("2z").code, ErrorCode::UnknownVariable);
}

// =====================================================================
// 6. Unit/currency conversion (UNIT-T001..T024)
// =====================================================================

#[test]
fn unit_t001_t002_length() {
    let mut a = App::new();
    assert_eq!(a.solve("1 meter in mm").unwrap(), "1000");
    assert_eq!(a.solve("1m in mm").unwrap(), "1000");
    assert_eq!(a.solve("1 inch in cm").unwrap(), "2.54");
}

#[test]
fn unit_t003_rack_units() {
    let mut a = App::new();
    assert_eq!(a.solve("44.45 mm in U").unwrap(), "1");
    assert_eq!(a.solve("1 U in inch").unwrap(), "1.75");
}

#[test]
fn unit_t004_temperature() {
    let mut a = App::new();
    assert_eq!(a.solve("100degC in degF").unwrap(), "212");
    assert_eq!(a.solve("0degC in degF").unwrap(), "32");
    assert_eq!(a.solve("0 K in degC").unwrap(), "−273.15");
    assert_eq!(a.solve("1 ˚C in ˚F").unwrap(), "33.8");
    assert_eq!(a.solve("-1 c in k").unwrap(), "272.15");
}

#[test]
fn unit_t005_t006_data() {
    let mut a = App::new();
    assert_eq!(a.solve("1000 bits in bytes").unwrap(), "125");
    assert_eq!(a.solve("1 kB in bits").unwrap(), "8000");
    assert_eq!(a.solve("1 GB in MiB").unwrap(), "953.674316406");
    assert_eq!(a.solve("10 kibibytes in b").unwrap(), "81920");
    assert_eq!(a.solve("1000 GiB in tebibytes").unwrap(), "0.9765625");
}

#[test]
fn unit_t007_energy() {
    let mut a = App::new();
    assert_eq!(a.solve("1000 Joule in kJ").unwrap(), "1");
    assert_eq!(a.solve("3.6 MJ in kilowatthour").unwrap(), "1");
    assert_eq!(a.solve("1 kWh in Megajoules").unwrap(), "3.6");
}

#[test]
fn unit_t008_pressure() {
    let mut a = App::new();
    assert_eq!(a.solve("1 inHg in pascal").unwrap(), "3386.38");
    assert_eq!(a.solve("101325 Pascal in inHg").unwrap(), "29.921331924");
}

#[test]
fn unit_t009_t010_t011_t012_misc() {
    let mut a = App::new();
    assert_eq!(a.solve("1 kg in pounds").unwrap(), "2.204622622");
    assert_eq!(a.solve("100 gradians in degrees").unwrap(), "90");
    assert_eq!(a.solve("1m² in mm²").unwrap(), "1000000");
    assert_eq!(a.solve("1m³ in mm³").unwrap(), "1000000000");
    assert_eq!(a.solve("1 minute in seconds").unwrap(), "60");
}

#[test]
fn unit_t013_bare_unit_and_missing_from() {
    let mut a = App::new();
    assert_eq!(a.err("5 m").code, ErrorCode::Invalid);
    assert_eq!(a.err("5 in mm").code, ErrorCode::Invalid);
}

#[test]
fn unit_t014_t015_unknown_and_cross_category() {
    let mut a = App::new();
    let e = a.err("5 x in y");
    assert!(
        matches!(
            e.code,
            ErrorCode::UnknownUnit | ErrorCode::UnknownConversion
        ),
        "{e:?}"
    );
    assert_eq!(a.err("1 meter in kg").code, ErrorCode::UnknownConversion);
}

#[test]
fn unit_t016_t017_t018_bare_words() {
    let mut a = App::new();
    assert_eq!(a.solve("g").unwrap(), "9.80665");
    assert_eq!(a.err("m").code, ErrorCode::UnknownVariable);
    assert_eq!(a.err("˚C").code, ErrorCode::Invalid);
}

#[test]
fn unit_t019_t020_t021_base_conversions() {
    let mut a = App::new();
    assert_eq!(a.solve("10 in bin").unwrap(), "1010₂");
    assert_eq!(a.solve("10 in hex").unwrap(), "A₁₆");
    assert_eq!(a.solve("10 in oct").unwrap(), "12₈");
    assert_eq!(a.solve("10 in dec").unwrap(), "10");
    a.solve("x=2").unwrap();
    assert_eq!(a.solve("x in bin").unwrap(), "10₂");
    assert_eq!(a.solve("0b1010 in dec").unwrap(), "10");
    assert_eq!(a.solve("0XA in dec").unwrap(), "10");
}

#[test]
fn unit_t022_t023_currency() {
    let mut a = App::new();
    a.currencies.set_rate("EUR", 1.0, "2026-10-30", "TEST");
    assert_eq!(a.solve("1 EUR in EUR").unwrap(), "1");
    // Offline: currency known but rate missing → UNKNOWN_RATE with message.
    let e = a.err("1 EUR in USD");
    assert_eq!(e.code, ErrorCode::UnknownRate);
    assert_eq!(e.message, "Exchange rate for “USD” is not available");
}

#[test]
fn unit_t024_large_base_conversion_grouped() {
    let mut a = App::new().display_grouping();
    let s = a
        .solve("1234567890123456789012345678901234567890 in bin")
        .unwrap();
    // The exact grouped binary of 1234567890123456789012345678901234567890
    // (130 digits → a 2-digit leading group, then groups of 4).
    let raw = "1110100000110010010010000001110101110000001101101111110011101110001010110010111100010111111001011011001110001111110000101011010010";
    let head = raw.len() % 4;
    let mut parts: Vec<&str> = Vec::new();
    if head > 0 {
        parts.push(&raw[..head]);
    }
    parts.extend(
        raw.as_bytes()[head..]
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap()),
    ); // Representation base differs from the parser base → trailing ₂ suffix
       // (doc 02 §8, decision D17).
    assert_eq!(s, format!("{}\u{2082}", parts.join("\u{202F}")));
}

// =====================================================================
// Financial formulas (doc 04 §4; UI-T019's engine side). The financial
// names are dialog-only (not in the expression catalog), so these go
// through radix_core::financial::evaluate — the dialog's entry point.
// =====================================================================

fn fin(name: &str, args: &[&str]) -> Result<String, radix_core::Error> {
    let args: Vec<radix_core::Number> = args
        .iter()
        .map(|s| {
            radix_core::Number::rational(
                radix_core::number::parse_decimal_rational(s).unwrap(),
                2000,
            )
        })
        .collect();
    let v = radix_core::financial::evaluate(name, &args)?;
    Serializer {
        format: NumberFormat::Fixed,
        show_thousands: false,
        ..Serializer::default()
    }
    .serialize(&v)
}

#[test]
fn financial_formulas() {
    // Compounding term: ln(fv/pv)/ln(1+pint) = ln 2 / ln 1.1 (UI-T019).
    assert_eq!(fin("Ctrm", &["0.1", "2", "1"]).unwrap(), "7.272540897");
    assert_eq!(fin("Fv", &["100", "0.05", "3"]).unwrap(), "315.25");
    assert_eq!(fin("Gpm", &["100", "0.2"]).unwrap(), "125");
    assert_eq!(fin("Pmt", &["1000", "0.05", "3"]).unwrap(), "367.208564631");
    assert_eq!(fin("Pv", &["100", "0.05", "3"]).unwrap(), "272.324802937");
    assert_eq!(fin("Rate", &["2", "1", "3"]).unwrap(), "0.25992105");
    assert_eq!(fin("Sln", &["1000", "100", "5"]).unwrap(), "180");
    assert_eq!(fin("Syd", &["1000", "100", "5", "1"]).unwrap(), "300");
    assert_eq!(fin("Syd", &["1000", "100", "5", "2"]).unwrap(), "240");
    // Double-declining: VAL = (cost − bv)·2/life iterated `period` times.
    assert_eq!(fin("Ddb", &["1000", "5", "1"]).unwrap(), "400");
    assert_eq!(fin("Ddb", &["1000", "5", "2"]).unwrap(), "240");
    assert_eq!(fin("Term", &["100", "1100", "0.1"]).unwrap(), "7.784450163");
}

#[test]
fn financial_domains() {
    // pint = 0 → division by zero for Fv/Pmt/Pv (doc 04 §4).
    let e = fin("Fv", &["100", "0", "3"]).unwrap_err();
    assert_eq!(e.code, ErrorCode::Mp);
    // fv/pv ≤ 0 → ln domain error for Ctrm.
    assert_eq!(
        fin("Ctrm", &["0.1", "0", "1"]).unwrap_err().code,
        ErrorCode::Mp
    );
    // Term with fv·pint/pmt ≤ −1 → ln domain error.
    assert_eq!(
        fin("Term", &["100", "-1000", "0.1"]).unwrap_err().code,
        ErrorCode::Mp
    );
    // Ddb's negative-period status: value computed with 0 iterations, error attached.
    let v = radix_core::financial::evaluate(
        "Ddb",
        &[
            radix_core::Number::int(1000, 2000),
            radix_core::Number::int(5, 2000),
            radix_core::Number::int(-1, 2000),
        ],
    )
    .unwrap();
    assert!(v.err.is_some());
    assert_eq!(v.to_integer().unwrap(), 0);
    // The names are dialog-only: not solvable from the expression catalog.
    let mut a = App::new();
    assert_eq!(a.err("Ctrm(0.1;2;1)").code, ErrorCode::UnknownFunction);
}
