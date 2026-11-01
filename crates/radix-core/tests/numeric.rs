//! Numeric engine tests (spec: doc 03, catalog doc 09 §3–4).
//!
//! Display-string assertions live in the serializer tests; here the engine
//! semantics are asserted via exact shadows, exact float equality, and
//! tolerance at display accuracy (9 digits).

use radix_core::bitwise::{self, ShiftKind};
use radix_core::number::{double_to_rational, Number};
use radix_core::numfuncs::{self, AngleUnit};
use rug::{Integer, Rational};

const P: u32 = 2000;

fn rat(s: &str) -> Number {
    Number::rational(radix_core::number::parse_decimal_rational(s).unwrap(), P)
}

fn int(v: i64) -> Number {
    Number::int(v, P)
}

fn f(v: f64) -> Number {
    Number::from_f64(v, P)
}

/// Approximate equality at display accuracy (9 fractional digits).
fn approx(a: &Number, expected: f64) -> bool {
    let av = a.re.to_f64();
    (av - expected).abs() <= 5e-10_f64.max(expected.abs() * 1e-15)
}

fn expect_value(n: &Number, expected: f64) {
    assert!(
        approx(n, expected),
        "expected {expected}, got {} (err: {:?})",
        n.re.to_f64(),
        n.err
    );
}

fn expect_err<T>(result: Result<T, radix_core::Error>, msg: &str) {
    match result {
        Ok(_) => panic!("expected error {msg}, got a value"),
        Err(e) => assert!(
            e.message.contains(msg) || e.code.name() == msg,
            "expected error containing {msg:?}, got {:?}",
            e
        ),
    }
}

// ----------------------------------------------------------------------
// Exact rational shadow (NUM-T001..005, T022)
// ----------------------------------------------------------------------

#[test]
fn num_t001_rational_add() {
    let r = rat("0.1").add(&rat("0.2"));
    assert_eq!(r.exact(), Some(&Rational::from((3, 10))));
    expect_value(&r, 0.3);
}

#[test]
fn num_t002_div_mul_exact() {
    let third = int(1).div(&int(3)).unwrap();
    assert_eq!(third.exact(), Some(&Rational::from((1, 3))));
    let r = third.mul(&int(3));
    assert_eq!(r.exact(), Some(&Rational::from(1)));
}

#[test]
fn num_t003_thirds_sum() {
    let r = int(2)
        .div(&int(3))
        .unwrap()
        .add(&int(1).div(&int(3)).unwrap());
    assert_eq!(r.exact(), Some(&Rational::from(1)));
}

#[test]
fn num_t004_cancellation() {
    let mut acc = rat("44.23");
    for v in ["8.10", "11.23", "6.9", "18"] {
        acc = acc.sub(&rat(v));
    }
    assert_eq!(acc.exact(), Some(&Rational::from(0)));
}

#[test]
fn num_t005_nineteen_digit_fractions() {
    let a = rat("0.3333333333333333333");
    let r = a.mul(&int(3)).sub(&int(1));
    assert_eq!(
        r.exact(),
        Some(&"-1/10000000000000000000".parse::<Rational>().unwrap())
    );
    // Three addends cancel exactly (doc 03 §1).
    let sum = a.add(&a).add(&rat("0.3333333333333333334")).sub(&int(1));
    assert_eq!(sum.exact(), Some(&Rational::from(0)));
}

#[test]
fn num_t022_big_scale_mult() {
    let r = int(1_000_000_000_000_000).mul(&rat("0.000000000000001"));
    assert_eq!(r.exact(), Some(&Rational::from(1)));
}

#[test]
fn shadow_dropped_by_transcendental() {
    let r = rat("0.5").sin(AngleUnit::Radians).unwrap();
    assert!(r.exact().is_none());
}

// ----------------------------------------------------------------------
// Division by zero (NUM-T006)
// ----------------------------------------------------------------------

#[test]
fn num_t006_div_by_zero() {
    expect_err(int(1).div(&int(0)), "Division by zero is undefined");
    expect_err(int(0).div(&int(0)), "Division by zero is undefined");
    expect_err(f(0.0).div(&f(0.0)), "Division by zero is undefined");
    // i ÷ 0 errors (complex numerator, zero divisor)
    let i = Number::complex(Number::zero(P).re.clone(), f(1.0).re);
    expect_err(i.div(&int(0)), "Division by zero is undefined");
}

// ----------------------------------------------------------------------
// Powers and roots (NUM-T007, EXPR-T029/030, FUNC-T015..020)
// ----------------------------------------------------------------------

#[test]
fn num_t007_pow_100() {
    let r = int(2).pow_op(&int(100)).unwrap();
    assert_eq!(
        r.to_integer().unwrap(),
        "1267650600228229401496703205376"
            .parse::<Integer>()
            .unwrap()
    );
}

#[test]
fn expr_t029_pow_zero_zero() {
    let r = int(0).pow_op(&int(0)).unwrap();
    assert!(r.is_zero());
    let err = r.err.expect("0^0 must carry an error");
    assert_eq!(err.message, "Zero raised to zero is undefined");
}

#[test]
fn expr_t030_pow_zero_frac() {
    let r = int(0).pow_op(&rat("0.5")).unwrap();
    assert!(r.is_zero());
    assert!(r.err.is_none());
}

#[test]
fn num_t024_pow_negative_zero_exp() {
    let r = int(0).pow_op(&int(-1)).unwrap();
    assert!(r.is_zero());
    let err = r.err.expect("0^-1 must carry an error");
    assert!(err.message.contains("power of zero"));
}

#[test]
fn func_t018_imaginary_unit() {
    let i = imaginary();
    let r = i.mul(&i);
    assert!(
        r.is_real() && r.re == -1 && r.im.is_zero(),
        "i×i must be exactly −1"
    );
    let inv = int(1).div(&i).unwrap();
    assert!(inv.is_complex());
    expect_value(&inv.im_part(), -1.0);
    let p = i.pow_op(&int(2)).unwrap();
    assert!(
        p.is_real() && p.re == -1 && p.im.is_zero(),
        "i^2 must be exactly −1"
    );
}

#[test]
fn func_t020_complex_roots() {
    // (−1)^0.5 = i
    let r = int(-1).pow_op(&rat("0.5")).unwrap();
    expect_value(&r.im_part(), 1.0);
    assert!(r.re_part().is_zero());
    // √−4 = 2i
    let r = int(-4).root(2).unwrap();
    expect_value(&r.im_part(), 2.0);
    assert!(r.re_part().is_zero());
    // e^iπ + 1 = 0 (display-level; here: real part −1 + 1, imag tiny)
    let ipi = imaginary().mul(&Number::pi(P));
    let e = ipi.exp().unwrap();
    let sum = e.add(&int(1));
    let re = sum.re_part();
    assert!(
        re.exact() == Some(&Rational::from(0)) || re.re.to_f64().abs() < 1e-9,
        "e^iπ+1 real part should be ~0"
    );
}

#[test]
fn func_t015_sqrt() {
    let r = int(2).sqrt().unwrap();
    expect_value(&r, std::f64::consts::SQRT_2);
    let r = int(-1).sqrt().unwrap();
    expect_value(&r.im_part(), 1.0);
    assert!(r.re_part().is_zero());
}

#[test]
fn root_negative_odd_real() {
    // ∛−8 = −2
    let r = int(-8).root(3).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-2)));
    // (−8)^(1÷3) via root path
    let r = int(-8).pow_op(&int(1).div(&int(3)).unwrap()).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-2)));
}

#[test]
fn expr_t026_nth_roots() {
    let r = int(8).root(3).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(2)));
    let r = int(1024).root(10).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(2)));
    expect_err(int(5).root(0), "zeroth root");
}

// ----------------------------------------------------------------------
// mod (NUM-T012)
// ----------------------------------------------------------------------

#[test]
fn num_t012_mod() {
    let cases = [
        (-1i64, 7i64, 6i64),
        (21, -9, -6),
        (-21, 9, 6),
        (-21, -9, -3),
        (0, 7, 0),
        (7, -9, -2),
    ];
    for (x, y, want) in cases {
        let r = int(x).mod_op(&int(y)).unwrap();
        assert_eq!(r.exact(), Some(&Rational::from(want)), "{x} mod {y}");
    }
    expect_err(int(1).mod_op(&imaginary()), "Modulus division");
}

// ----------------------------------------------------------------------
// Rounding family (NUM-T013..015)
// ----------------------------------------------------------------------

#[test]
fn num_t013_rounding_family() {
    // round: ties away from zero
    let r = rat("3.5").round().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(4)));
    let r = rat("-3.5").round().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-4)));
    // floor / ceiling
    let r = rat("-3.2").floor().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-4)));
    let r = rat("-3.2").ceil().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-3)));
    let r = rat("3.5").floor().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(3)));
    let r = rat("3.5").ceil().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(4)));
}

#[test]
fn num_t014_fractional_part() {
    let r = rat("-3.2").fractional_part().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from((4, 5))));
    let r = rat("3.2").fractional_part().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from((1, 5))));
}

#[test]
fn num_t015_frac_component() {
    let r = rat("-3.2").frac_component().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from((-1, 5))));
}

// ----------------------------------------------------------------------
// abs / sgn (NUM-T016..017, T018)
// ----------------------------------------------------------------------

fn imaginary() -> Number {
    let mut n = Number::zero(P);
    n.im = Number::int(1, P).re;
    n
}

#[test]
fn num_t016_abs() {
    let r = int(3).sub(&int(5)).abs();
    assert_eq!(r.exact(), Some(&Rational::from(2)));
    let r = imaginary().abs();
    expect_value(&r, 1.0);
    let z = Number::complex(int(3).re, int(4).re);
    let r = z.abs();
    expect_value(&r, 5.0);
}

#[test]
fn num_t017_sgn() {
    let r = int(-3).sgn().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-1)));
    let r = int(0).sgn().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(0)));
    let r = int(7).sgn().unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(1)));
}

#[test]
fn num_t018_negative_zero() {
    // −0÷32352.689 = 0
    let nz = f(-0.0);
    let r = nz.div(&rat("32352.689")).unwrap();
    assert!(r.is_zero());
    assert_eq!(r.exact(), Some(&Rational::from(0)));
}

// ----------------------------------------------------------------------
// Factorial and combinatorics (NUM-T008..011, T019)
// ----------------------------------------------------------------------

#[test]
fn num_t008_factorial() {
    let r = int(0).factorial().unwrap();
    assert!(r.re == 1, "0! must be exactly 1");
    let r = int(5).factorial().unwrap();
    assert!(r.re == 120, "5! must be exactly 120");
    // 69! — 108 digits, exact at 2000 bits.
    let r = int(69).factorial().unwrap();
    let s = r.re.clone().to_string();
    let s = s.trim_end_matches('0');
    let expected = "171122452428141311372468338881272839092270544893520369393648040923257279754140647424000000000000000";
    assert!(s.starts_with(expected), "69! prefix mismatch: {s}");
}

#[test]
fn num_t009_gamma_factorial() {
    let r = rat("3.5").factorial().unwrap();
    expect_value(&r, 11.631728396567448);
    let r = rat("-10.9").factorial().unwrap();
    expect_value(&r, 0.0000035426845530808342);
}

#[test]
fn num_t010_factorial_poles() {
    expect_err(int(-1).factorial(), "negative integers");
    expect_err(rat("-10.0").factorial(), "negative integers");
    expect_err(imaginary().factorial(), "complex");
}

#[test]
fn num_t019_ncr_npr() {
    let r = int(9).ncr(&int(5)).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(126)));
    let r = int(9).npr(&int(5)).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(15120)));
    expect_err(int(9).ncr(&int(10)), "n is less than r");
    expect_err(int(9).ncr(&int(-5)), "non-negative");
    let r = int(9).npr(&int(0)).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(1)));
}

// ----------------------------------------------------------------------
// gcd / lcm (NUM-T020..021)
// ----------------------------------------------------------------------

#[test]
fn num_t020_gcd_lcm() {
    let g = numfuncs::gcd(&[int(456), int(1584)]).unwrap();
    assert_eq!(g.exact(), Some(&Rational::from(24)));
    let g = numfuncs::gcd(&[int(456), int(1584), int(40), int(60)]).unwrap();
    assert_eq!(g.exact(), Some(&Rational::from(4)));
    let l = numfuncs::lcm(&[int(456), int(1584)]).unwrap();
    assert_eq!(l.exact(), Some(&Rational::from(30096)));
    let g = numfuncs::gcd(&[int(5)]).unwrap();
    assert_eq!(g.exact(), Some(&Rational::from(5)));
}

#[test]
fn num_t021_gcd_noninteger() {
    expect_err(
        numfuncs::gcd(&[int(456), rat("15.84")]),
        "only defined for integers",
    );
}

// ----------------------------------------------------------------------
// Trigonometry with exactness guards (FUNC-T001..009)
// ----------------------------------------------------------------------

#[test]
fn func_t001_sin_exact() {
    let r = int(90).sin(AngleUnit::Degrees).unwrap();
    expect_value(&r, 1.0);
    let r = int(180).sin(AngleUnit::Degrees).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(0)));
}

#[test]
fn func_t002_cos_exact() {
    let r = int(90).cos(AngleUnit::Degrees).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(0)));
    let r = int(180).cos(AngleUnit::Degrees).unwrap();
    expect_value(&r, -1.0);
}

#[test]
fn func_t003_tan_undefined() {
    let e = int(90).tan(AngleUnit::Degrees).unwrap_err();
    assert!(e.message.starts_with("Tangent is undefined"), "{:?}", e);
    // tan 180 = 0 exact
    let r = int(180).tan(AngleUnit::Degrees).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(0)));
}

#[test]
fn func_t004_sin_identity() {
    // sin 45 − 1÷√2 = 0
    let a = int(45).sin(AngleUnit::Degrees).unwrap();
    let b = int(1).div(&int(2).sqrt().unwrap()).unwrap();
    let d = a.sub(&b);
    assert!(d.re.to_f64().abs() < 1e-9, "difference: {}", d.re.to_f64());
}

#[test]
fn func_t005_squared() {
    let r = int(45)
        .sin(AngleUnit::Degrees)
        .unwrap()
        .pow_op(&int(2))
        .unwrap();
    expect_value(&r, 0.5);
    let r = int(10)
        .tan(AngleUnit::Degrees)
        .unwrap()
        .pow_op(&int(2))
        .unwrap();
    expect_value(&r, 0.031091204);
}

#[test]
fn func_t006_angle_units() {
    let r = Number::pi(P)
        .div(&int(2))
        .unwrap()
        .sin(AngleUnit::Radians)
        .unwrap();
    expect_value(&r, 1.0);
    let r = int(100).sin(AngleUnit::Gradians).unwrap();
    expect_value(&r, 1.0);
}

#[test]
fn func_t007_inverse_trig() {
    let r = int(0).acos(AngleUnit::Degrees).unwrap();
    expect_value(&r, 90.0);
    let r = int(1).asin(AngleUnit::Degrees).unwrap();
    expect_value(&r, 90.0);
    let r = int(-1).asin(AngleUnit::Degrees).unwrap();
    expect_value(&r, -90.0);
    expect_err(rat("2").asin(AngleUnit::Degrees), "Inverse sine");
    expect_err(rat("2").acos(AngleUnit::Degrees), "Inverse cosine");
}

#[test]
fn func_t008_alias_asin() {
    // asin is registered as a separate entry to the same op (parser-level);
    // engine-level equivalence:
    let r = int(1).asin(AngleUnit::Degrees).unwrap();
    expect_value(&r, 90.0);
}

#[test]
fn func_t010_atanh_identity() {
    // atanh(1÷10) − 0.5·ln(11÷9) = 0
    let a = int(1).div(&int(10)).unwrap().atanh().unwrap();
    let b = int(11).div(&int(9)).unwrap().ln().unwrap();
    let b = b.mul(&rat("0.5"));
    let d = a.sub(&b);
    assert!(d.re.to_f64().abs() < 1e-9, "{}", d.re.to_f64());
}

#[test]
fn func_t011_cosh_sinh() {
    // cosh 10 − (e^10 + e^−10)/2 = 0
    let c = int(10).cosh().unwrap();
    let e10 = int(10).exp().unwrap();
    let en10 = int(-10).exp().unwrap();
    let m = e10.add(&en10).div(&int(2)).unwrap();
    let d = c.sub(&m);
    assert!(d.re.to_f64().abs() < 1e-9, "{}", d.re.to_f64());
    // sinh(−10) + sinh 10 = 0
    let d = int(-10).sinh().unwrap().add(&int(10).sinh().unwrap());
    assert!(d.re.to_f64().abs() < 1e-9);
    expect_err(rat("0.5").acosh(), "less than one");
    expect_err(int(1).atanh(), "outside [-1, 1]");
}

// ----------------------------------------------------------------------
// Logs (FUNC-T012..014, T022)
// ----------------------------------------------------------------------

#[test]
#[allow(clippy::approx_constant)] // decimal literals are the spec, not f64 consts
fn func_t012_log() {
    let r = int(10).log(None).unwrap();
    expect_value(&r, 1.0);
    let r = int(2).log(None).unwrap();
    expect_value(&r, 0.301029995663981);
    let r = int(10).log(Some(&int(10))).unwrap();
    expect_value(&r, 1.0);
    let r = int(2).log(Some(&int(2))).unwrap();
    expect_value(&r, 1.0);
}

#[test]
#[allow(clippy::approx_constant)]
fn func_t013_ln() {
    let r = int(1).ln().unwrap();
    assert!(r.re.is_zero(), "ln 1 must be exactly 0");
    let r = int(2).ln().unwrap();
    expect_value(&r, 0.693147180559945);
    let e = f(std::f64::consts::E);
    let r = e.ln().unwrap();
    assert!((r.re.to_f64() - 1.0).abs() < 1e-9);
}

#[test]
fn func_t014_log_domains() {
    expect_err(int(0).ln(), "Logarithm of zero");
    expect_err(int(0).log(None), "Logarithm of zero");
    expect_err(int(2).log(Some(&int(0))), "base 0 or 1");
    expect_err(int(2).log(Some(&int(1))), "base 0 or 1");
}

#[test]
fn func_t022_complex_logs() {
    // log(−10) − (1 + πi÷ln 10) = 0
    let a = int(-10).log(None).unwrap();
    let want_im = Number::pi(P).div(&int(10).ln().unwrap()).unwrap();
    assert!((a.re_part().re.to_f64() - 1.0).abs() < 1e-9);
    let d = a.im_part().sub(&want_im);
    assert!(d.re.to_f64().abs() < 1e-9);
    // ln(−e) − (1 + πi) = 0
    let a = f(std::f64::consts::E).neg().ln().unwrap();
    assert!((a.re_part().re.to_f64() - 1.0).abs() < 1e-9);
    let d = a.im_part().sub(&Number::pi(P));
    assert!(d.re.to_f64().abs() < 1e-9);
}

// ----------------------------------------------------------------------
// Complex (FUNC-T018..021, T023)
// ----------------------------------------------------------------------

#[test]
fn func_t019_complex_display_values() {
    // engine-level: parts round-trip
    let z = Number::complex(int(1).re, int(1).re);
    assert!(z.is_complex());
    assert!(!z.is_real());
}

#[test]
fn func_t021_arg() {
    let z = Number::complex(int(1).re, int(1).re);
    let r = z.arg(AngleUnit::Degrees).unwrap();
    expect_value(&r, 45.0);
    let r = int(-1).arg(AngleUnit::Degrees).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(180)));
    let zi = Number::complex(Number::zero(P).re, int(-1).re);
    let r = zi.arg(AngleUnit::Degrees).unwrap();
    expect_value(&r, -90.0);
    expect_err(
        int(0).arg(AngleUnit::Degrees),
        "Argument not defined for zero",
    );
}

#[test]
fn func_t023_complex_trig_identity() {
    // sin(iπ/4) − i·sinh(π/4) = 0
    let ipi4 = imaginary().mul(&Number::pi(P).div(&int(4)).unwrap());
    let a = ipi4.sin(AngleUnit::Radians).unwrap();
    let b = Number::pi(P).div(&int(4)).unwrap().sinh().unwrap();
    let b = imaginary().mul(&b);
    let d = a.sub(&b);
    assert!(d.re.to_f64().abs() < 1e-9 && d.im.to_f64().abs() < 1e-9);
    // cos(iπ/4) − cosh(π/4) = 0
    let a = ipi4.cos(AngleUnit::Radians).unwrap();
    let b = Number::pi(P).div(&int(4)).unwrap().cosh().unwrap();
    let d = a.sub(&b);
    assert!(d.re.to_f64().abs() < 1e-9 && d.im.to_f64().abs() < 1e-9);
}

// ----------------------------------------------------------------------
// Bitwise and shifts (NUM-T024..028)
// ----------------------------------------------------------------------

fn hex(s: &str, prec: u32) -> Number {
    Number::integer(Integer::from_str_radix(s, 16).unwrap(), prec)
}

fn bin(s: &str, prec: u32) -> Number {
    Number::integer(Integer::from_str_radix(s, 2).unwrap(), prec)
}

#[test]
fn num_t024_twos() {
    let r = bitwise::twos(&hex("1", P), 32).unwrap();
    assert_eq!(
        r.to_integer().unwrap().to_string_radix(16).to_uppercase(),
        "FFFFFFFF"
    );
    let r = bitwise::twos(&hex("7FFFFFFF", P), 32).unwrap();
    assert_eq!(
        r.to_integer().unwrap().to_string_radix(16).to_uppercase(),
        "80000001"
    );
}

#[test]
fn num_t025_not() {
    let r = bitwise::not(&hex("1", P), 32).unwrap();
    assert_eq!(
        r.to_integer().unwrap().to_string_radix(16).to_uppercase(),
        "FFFFFFFE"
    );
    let r = bitwise::not(&hex("7A", P), 32).unwrap();
    assert_eq!(
        r.to_integer().unwrap().to_string_radix(16).to_uppercase(),
        "FFFFFF85"
    );
}

#[test]
fn num_t026_bitwise_wordlen4() {
    let a = bin("1100", P);
    let b = bin("1010", P);
    let to_bin = |n: &Number| n.to_integer().unwrap().to_string_radix(2);
    assert_eq!(to_bin(&bitwise::and(&a, &b).unwrap()), "1000");
    // NAND: not(1000) masked to 4 bits = 0111 → 111 when printed
    assert_eq!(to_bin(&bitwise::nand(&a, &b, 4).unwrap()), "111");
    // NOR: not(1110) = 0001
    assert_eq!(to_bin(&bitwise::nor(&a, &b, 4).unwrap()), "1");
    // XNOR: not(0110) = 1001
    assert_eq!(to_bin(&bitwise::xnor(&a, &b, 4).unwrap()), "1001");
    expect_err(bitwise::and(&rat("1.5"), &int(1)), "non-negative integers");
    expect_err(bitwise::and(&int(-1), &int(1)), "non-negative integers");
}

#[test]
fn num_t027_shifts() {
    let sh = |a: i64, c: i64, w: u32, k: ShiftKind| {
        bitwise::shift(&int(a), &int(c), w, k)
            .unwrap()
            .to_integer()
            .unwrap()
    };
    assert_eq!(sh(1, 2, 64, ShiftKind::Left), Integer::from(4));
    // 1«2»1«2 = 8
    let t = bitwise::shift(
        &bitwise::shift(&int(1), &int(2), 64, ShiftKind::Left).unwrap(),
        &int(1),
        64,
        ShiftKind::Right,
    )
    .unwrap();
    let t = bitwise::shift(&t, &int(2), 64, ShiftKind::Left).unwrap();
    assert_eq!(t.to_integer().unwrap(), Integer::from(8));
    assert_eq!(sh(1, 35, 64, ShiftKind::Left).to_string(), "34359738368");
    assert_eq!(
        sh(1, 63, 64, ShiftKind::Left).to_string(),
        "9223372036854775808"
    );
}

#[test]
fn num_t028_shift_overflow() {
    // Count reduces mod word size: 1«70 (wl 64) = 1«6 = 64.
    let r = bitwise::shift(&int(1), &int(70), 64, ShiftKind::Left).unwrap();
    assert_eq!(r.to_integer().unwrap(), Integer::from(64));
    // A value above the unsigned word range overflows.
    let big = Number::integer(Integer::from(1) << 64u32, P);
    let e = bitwise::shift(&big, &int(1), 64, ShiftKind::Left).unwrap_err();
    assert_eq!(e.message, "Overflow. Try a bigger word size");
    // A value below the signed range overflows.
    let m: Integer = (Integer::from(1) << 63u32) + 1;
    let small = Number::integer(-m, P);
    expect_err(
        bitwise::shift(&small, &int(1), 64, ShiftKind::Left),
        "Overflow",
    );
    expect_err(
        bitwise::shift(&rat("1.5"), &int(1), 64, ShiftKind::Left),
        "Shift is only possible on integers",
    );
}

#[test]
fn word_reinterpretation() {
    // 255 at 8-bit → −1; −1 at 64 → −1 at 8
    let r = bitwise::reinterpret_word_size(&int(255), 8);
    assert_eq!(r.to_integer().unwrap(), Integer::from(-1));
    let r = bitwise::reinterpret_word_size(&int(-1), 8);
    assert_eq!(r.to_integer().unwrap(), Integer::from(-1));
    let r = bitwise::reinterpret_word_size(&int(200), 8);
    assert_eq!(r.to_integer().unwrap(), Integer::from(-56));
    // floats left alone
    let r = bitwise::reinterpret_word_size(&rat("1.5"), 8);
    assert_eq!(r.exact(), Some(&Rational::from((3, 2))));
}

// ----------------------------------------------------------------------
// Statistics and other functions
// ----------------------------------------------------------------------

#[test]
fn statistics() {
    let args = [int(1), int(2), int(3), int(4)];
    let r = Number::sum(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(10)));
    let r = Number::sumsq(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(30)));
    let r = Number::average(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from((5, 2))));
    let r = Number::median(&[int(1), int(9), int(2)]).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(2)));
    let r = Number::median(&[int(1), int(9), int(2), int(4)]).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(3)));
    let r = Number::min(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(1)));
    let r = Number::max(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(4)));
    // var of 1,2,3,4 = 5/3? sample var = Σ(x−2.5)²/3 = (2.25+0.25+0.25+2.25)/3 = 5/3
    let r = Number::var(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from((5, 3))));
    let r = Number::varp(&args).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from((5, 4))));
    expect_err(Number::var(&[int(1)]), "takes at least 2");
    expect_err(Number::median(&[imaginary()]), "Median is only defined");
}

#[test]
fn modexp_cases() {
    // 7^11 mod 13 = 2
    let r = int(7).modexp(&int(11), &int(13)).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(2)));
    // negative exponent: 3^(−1) mod 7 = 5 (3·5=15≡1)
    let r = int(3).modexp(&int(-1), &int(7)).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(5)));
    // negative modulus
    let r = int(7).modexp(&int(11), &int(-13)).unwrap();
    assert_eq!(r.exact(), Some(&Rational::from(-11)));
}

#[test]
fn factorize_cases() {
    use radix_core::bitwise::factorize;
    let fs = factorize(&int(360)).unwrap();
    assert_eq!(
        fs.iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(","),
        "2,2,2,3,3,5"
    );
    let fs = factorize(&int(-12)).unwrap();
    assert_eq!(
        fs.iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(","),
        "-2,2,3"
    );
    let fs = factorize(&int(0)).unwrap();
    assert_eq!(fs.len(), 1);
    assert!(fs[0].is_zero());
    let fs = factorize(&int(1)).unwrap();
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0], Integer::from(1));
    let fs = factorize(&int(97)).unwrap();
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0], Integer::from(97));
    expect_err(
        radix_core::bitwise::factorize(&rat("1.5")),
        "Need an integer",
    );
}

// ----------------------------------------------------------------------
// Precision and misc
// ----------------------------------------------------------------------

#[test]
fn precision_model() {
    // Display precision independent of working precision: stored value keeps
    // full precision (doc 03 §2). 1÷3 keeps the shadow; the float is 2000-bit.
    let third = int(1).div(&int(3)).unwrap();
    assert_eq!(third.prec(), P);
    assert!(third.exact().is_some());
}

#[test]
fn rand_range() {
    for _ in 0..20 {
        let r = radix_core::number::random_unit(P);
        let v = r.re.to_f64();
        assert!((0.0..1.0).contains(&v), "rand out of range: {v}");
    }
}

#[test]
fn double_to_rational_exact() {
    let r = double_to_rational(0.5);
    assert_eq!(r, Rational::from((1, 2)));
    let r = double_to_rational(0.1);
    // 0.1 as double = 3602879701896397 / 36028797018963968
    assert_eq!(
        r,
        "3602879701896397/36028797018963968"
            .parse::<Rational>()
            .unwrap()
    );
    let r = double_to_rational(-2.75);
    assert_eq!(r, Rational::from((-11, 4)));
}

#[test]
fn bswap_cases() {
    let r = bitwise::bswap(&hex("12345678", P), 32).unwrap();
    assert_eq!(
        r.to_integer().unwrap().to_string_radix(16).to_uppercase(),
        "78563412"
    );
    let r = bitwise::bswap(&int(-1), 32).unwrap();
    assert_eq!(r.to_integer().unwrap(), Integer::from(-1));
}
