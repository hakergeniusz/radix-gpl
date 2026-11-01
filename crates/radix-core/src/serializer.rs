//! Display serialization (spec: doc 07).
//!
//! Turns a [`Number`] into the exact byte sequence the display shows: the
//! four number formats with half-away-from-zero rounding at `accuracy`
//! fractional digits, the per-base automatic thresholds (doc 07 §2.2),
//! per-base grouping (3 in decimal, 4 with a narrow space otherwise), the
//! fixed-format 2^P cap, base-suffix subscripts, the complex `a+bi` rules,
//! and the DMS / register auxiliaries.
//!
//! All digit extraction is exact: a value is either its rational shadow
//! (`p/q`, scaled by integer arithmetic) or its binary expansion
//! (`m × 2^e` from MPFR, scaled by shifts), so truncating `|v| · b^k` never
//! goes through floating point.

use crate::error::{messages, Error};
use crate::number::Number;
use rug::ops::Pow;
use rug::{Float, Integer, Rational};

/// Grouping separator for non-decimal bases (doc 07 §1: U+202F).
pub const NARROW_SPACE: char = '\u{202F}';
/// Unicode minus — the only minus sign ever emitted (doc 07 §3).
pub const MINUS: char = '\u{2212}';

/// Display digit glyphs; the default build uses ASCII digits with uppercase
/// hex letters (doc 07 §6, `use_default_digits = true`).
const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
/// Superscript digits for exponents.
const SUP: &[char; 10] = &['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
/// Superscript letters for digit values 10–15 in non-decimal exponents
/// (doc 07 §2.5, v51 "show exponent in the current base").
const SUP_LETTERS: &[char; 6] = &[
    '\u{1D2C}', '\u{1D2E}', '\u{1D9C}', '\u{1D30}', '\u{1D31}', '\u{1DA0}',
];
/// Superscript minus (doc 07 §2.3).
const SUP_MINUS: char = '⁻';
/// Subscript digits for base suffixes (`1010₂`, `A₁₆`).
const SUB: &[char; 10] = &['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];

/// Superscript glyph for a digit value 0–9 (GUI superscript entry mode).
pub fn superscript_digit(v: char) -> char {
    match v.to_digit(10) {
        Some(d) => SUP[d as usize],
        None => v,
    }
}

/// Subscript glyph for a digit value 0–9 (GUI subscript entry mode).
pub fn subscript_digit(v: char) -> char {
    match v.to_digit(10) {
        Some(d) => SUB[d as usize],
        None => v,
    }
}

/// Superscript-minus glyph (GUI exponent entry, Ctrl+E).
pub const SUPERSCRIPT_MINUS: char = SUP_MINUS;

/// The `number-format` setting (doc 07 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberFormat {
    Automatic,
    Fixed,
    Scientific,
    Engineering,
}

/// Display serializer (doc 07 §1–3). One instance holds the current
/// formatting settings; `serialize` renders values under them.
#[derive(Debug, Clone)]
pub struct Serializer {
    /// `accuracy`: fractional digits kept by the fixed rendering (A).
    pub accuracy: u32,
    /// `show-thousands`: grouping separators.
    pub show_thousands: bool,
    /// `show-zeroes`: pad the fraction to A digits.
    pub show_zeroes: bool,
    /// `number-format`.
    pub format: NumberFormat,
    /// Locale radix character (doc 07 §1).
    pub radix: char,
    /// Locale thousands separator (doc 07 §1).
    pub thousands: char,
    /// Representation base — the base results display in (doc 02 §12.2).
    pub repr_base: u32,
    /// Base the expression was parsed in; the base suffix appears only when
    /// it differs from `repr_base` (doc 07 §2.5).
    pub parser_base: u32,
    /// `leading_digits` automatic threshold for base 10 (doc 07 §2.2, OQ-03).
    pub leading_digits: u32,
}

impl Default for Serializer {
    fn default() -> Self {
        Serializer {
            accuracy: 9,
            show_thousands: true,
            show_zeroes: false,
            format: NumberFormat::Automatic,
            radix: '.',
            thousands: ',',
            repr_base: 10,
            parser_base: 10,
            leading_digits: 12,
        }
    }
}

/// Exact magnitude source for digit extraction: `p/q` (shadow, q > 0, p ≥ 0)
/// or `m × 2^e` (binary expansion, m > 0).
enum Src {
    Rat(Integer, Integer),
    Bin(Integer, i32),
}

impl Serializer {
    // ------------------------------------------------------------------
    // Public entry points
    // ------------------------------------------------------------------

    /// Render a value for the display / CLI answer (doc 07 §2).
    pub fn serialize(&self, v: &Number) -> Result<String, Error> {
        if !v.is_finite() {
            return Ok(self.serialize_nonfinite(v));
        }
        if v.is_complex() {
            return self.serialize_complex(v);
        }
        let (s, is_fixed) = self.render_real(v)?;
        // Base suffix on fixed renderings when the representation base
        // differs from the parser base (doc 07 §2.5, FMT-T013/T014).
        if is_fixed && self.repr_base != self.parser_base {
            return Ok(format!("{s}{}", self.base_suffix()));
        }
        Ok(s)
    }

    /// DMS rendering `d°m′s″` of a degree value (doc 07 §2.7). Rounding the
    /// total seconds half-away at the accuracy scale gives the documented
    /// carry behavior (60″ → +1′, 60′ → +1°) and snaps near-integer seconds.
    pub fn serialize_dms(&self, v: &Number) -> String {
        if !v.is_finite() {
            return self.serialize_nonfinite(v);
        }
        let neg = v.re < 0;
        let src = match Self::source(v) {
            None => return "0°0′0″".to_string(),
            Some(s) => s,
        };
        // total = round(|v| · 3600 · 10^A), exact — one half-away rounding of
        // the total seconds, so minutes/seconds stay in range by construction
        // and near-integer seconds snap (doc 07 §2.7).
        let mult = Integer::from(3600) * Integer::from(10).pow(self.accuracy);
        let total = self.round_scaled(&src, &mult);
        let unit = Integer::from(10).pow(self.accuracy);
        let per_min = Integer::from(60) * unit.clone();
        let per_deg = Integer::from(60) * per_min.clone();
        let d = total.clone() / per_deg.clone();
        let rem = total % per_deg;
        let m = rem.clone() / per_min.clone();
        let s = rem % per_min;
        let s_int = s.clone() / unit.clone();
        let s_frac = self.frac_string(&(s % unit), self.accuracy);
        let mut out = String::new();
        if neg {
            out.push(MINUS);
        }
        out.push_str(&self.digit_string(&d));
        out.push('°');
        out.push_str(&self.digit_string(&m));
        out.push('′');
        out.push_str(&self.digit_string(&s_int));
        if !s_frac.is_empty() {
            out.push(self.radix);
            out.push_str(&s_frac);
        }
        out.push('″');
        out
    }

    /// Register-file value string (doc 08 §2): scientific format, radix `.`,
    /// no grouping, up to ~50 fractional digits.
    pub fn register_string(v: &Number) -> String {
        let s = Serializer {
            accuracy: 50,
            show_thousands: false,
            show_zeroes: false,
            format: NumberFormat::Scientific,
            radix: '.',
            thousands: ',',
            repr_base: 10,
            parser_base: 10,
            leading_digits: 12,
        };
        s.serialize(v).unwrap_or_default()
    }

    // ------------------------------------------------------------------
    // Format dispatch (doc 07 §2)
    // ------------------------------------------------------------------

    /// Render a real value; the bool reports whether the rendering is a fixed
    /// rendering (base suffix applies). The imaginary field of `v` is ignored,
    /// so this doubles as the complex part renderer.
    fn render_real(&self, v: &Number) -> Result<(String, bool), Error> {
        let src = match Self::source(v) {
            None => return Ok((self.zero_string(), true)),
            Some(s) => s,
        };
        let neg = v.re < 0;
        match self.format {
            NumberFormat::Fixed => {
                self.cap_check(v)?;
                Ok((self.fixed_str(&src, neg)?, true))
            }
            NumberFormat::Automatic => {
                self.cap_check(v)?;
                let (int_s, frac_s) = self.fixed_digits(&src)?;
                // Automatic threshold (doc 07 §2.2, decision D8): base 10
                // counts integer-part digits, other bases total digits.
                let over = if self.repr_base == 10 {
                    int_s.len() > self.leading_digits as usize
                } else {
                    int_s.len() + frac_s.len() > self.auto_threshold() as usize
                };
                if over {
                    Ok((self.scientific_str(&src, neg, false)?, false))
                } else {
                    Ok((self.assemble_fixed(&int_s, &frac_s, neg), true))
                }
            }
            // Explicit scientific/engineering in a non-decimal base falls
            // back to fixed with grouping (doc 07 §2.3, FMT-T011); the
            // automatic overflow keeps base-rendered exponents (§2.5).
            NumberFormat::Scientific | NumberFormat::Engineering => {
                if self.repr_base == 10 {
                    let eng = self.format == NumberFormat::Engineering;
                    Ok((self.scientific_str(&src, neg, eng)?, false))
                } else {
                    self.cap_check(v)?;
                    Ok((self.fixed_str(&src, neg)?, true))
                }
            }
        }
    }

    /// Fixed/automatic cap: |v| ≥ 2^precision cannot render (doc 07 §2.1);
    /// scientific and engineering are exempt (doc 03 §2).
    fn cap_check(&self, v: &Number) -> Result<(), Error> {
        if v.magnitude_geq_power_of_two(v.prec()) {
            return Err(Error::mp(messages::FIXED_FORMAT_TOO_LONG));
        }
        Ok(())
    }

    /// Automatic threshold by representation base (doc 07 §2.2).
    fn auto_threshold(&self) -> u32 {
        match self.repr_base {
            2 => 64,
            8 => 22,
            16 => 16,
            _ => self.leading_digits,
        }
    }

    // ------------------------------------------------------------------
    // Fixed rendering (doc 07 §2.1)
    // ------------------------------------------------------------------

    /// Fixed rendering from an exact source. Rounding is half away from
    /// zero at digit A+1: `q = floor(|v|·b^A + 1/2) = (2·T + b) / (2·b)`
    /// with `T = floor(|v|·b^(A+1))` (exact for even bases 2/8/10/16).
    fn fixed_digits(&self, src: &Src) -> Result<(String, String), Error> {
        let b = self.repr_base;
        let a = self.accuracy as i64;
        let t = self.trunc_scaled(src, a + 1);
        let bi = Integer::from(b);
        let q = (Integer::from(2) * t + bi.clone()) / (Integer::from(2) * bi.clone());
        let ba = Integer::from(b).pow(self.accuracy);
        let int = q.clone() / ba.clone();
        let frac = q % ba;
        Ok((
            self.digit_string(&int),
            self.frac_string(&frac, self.accuracy),
        ))
    }

    fn fixed_str(&self, src: &Src, neg: bool) -> Result<String, Error> {
        let (int_s, frac_s) = self.fixed_digits(src)?;
        Ok(self.assemble_fixed(&int_s, &frac_s, neg))
    }

    fn assemble_fixed(&self, int_s: &str, frac_s: &str, neg: bool) -> String {
        // A value that rounds to zero renders without a sign (doc 07 §3).
        let neg = neg && !(int_s == "0" && frac_s.is_empty());
        let mut out = String::new();
        if neg {
            out.push(MINUS);
        }
        out.push_str(&self.group(int_s));
        if !frac_s.is_empty() {
            out.push(self.radix);
            out.push_str(frac_s);
        }
        out
    }

    // ------------------------------------------------------------------
    // Scientific / engineering rendering (doc 07 §2.3, §2.4)
    // ------------------------------------------------------------------

    /// Mantissa in [1, b) (engineering: [1, b³)) with A fractional digits,
    /// then `×10` with the exponent as superscript digits in the current
    /// base; no exponent for exponent 0. Works for every base 2–16; the
    /// non-decimal explicit formats fall back to fixed before getting here.
    fn scientific_str(&self, src: &Src, neg: bool, engineering: bool) -> Result<String, Error> {
        let b = self.repr_base;
        let a = self.accuracy as i64;

        // Exponent e10: |v| / b^e10 ∈ [1, b). Estimated from the bit size,
        // then corrected with exact integer comparisons.
        let log2_est = match src {
            Src::Rat(p, q) => p.significant_bits() as i64 - q.significant_bits() as i64,
            Src::Bin(m, e) => *e as i64 + m.significant_bits() as i64 - 1,
        };
        let est = (log2_est as f64 / (b as f64).log2()).floor() as i64;
        let fv = self.trunc_scaled(src, 0);
        let mut e10 = if !fv.is_zero() {
            let mut e = est.max(0);
            while fv >= Integer::from(b).pow((e + 1) as u32) {
                e += 1;
            }
            while e > 0 && fv < Integer::from(b).pow(e as u32) {
                e -= 1;
            }
            e
        } else {
            let mut k = (-est).max(0);
            while self.trunc_scaled(src, k).is_zero() {
                k += 1;
            }
            while k > 0 && !self.trunc_scaled(src, k - 1).is_zero() {
                k -= 1;
            }
            -k
        };

        // Engineering: exponent rounded down to a multiple of 3, so the
        // mantissa spans [1, b³) (doc 07 §2.4).
        if engineering {
            e10 -= ((e10 % 3) + 3) % 3;
        }

        // Rounded mantissa scaled by b^A, half away from zero.
        let t = self.trunc_scaled(src, a + 1 - e10);
        let bi = Integer::from(b);
        let mut q = (Integer::from(2) * t + bi.clone()) / (Integer::from(2) * bi.clone());
        // Renormalize when rounding carries into the next mantissa digit
        // (doc 07 §2.4: 10… → exponent+1, 1000… → exponent+3).
        let (fix, lim) = if engineering { (3, a + 3) } else { (1, a + 1) };
        if q >= Integer::from(b).pow(lim as u32) {
            q /= Integer::from(b).pow(fix as u32);
            e10 += fix;
        }

        let ba = Integer::from(b).pow(self.accuracy);
        let mint = q.clone() / ba.clone();
        let mfrac = q % ba;
        let mut out = String::new();
        if neg {
            out.push(MINUS);
        }
        out.push_str(&self.digit_string(&mint));
        let fs = self.frac_string(&mfrac, self.accuracy);
        if !fs.is_empty() {
            out.push(self.radix);
            out.push_str(&fs);
        }
        if e10 != 0 {
            out.push_str("×10");
            out.push_str(&self.superscript_exponent(e10));
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Special values and complex (doc 07 §2.6, FMT-T017)
    // ------------------------------------------------------------------

    fn serialize_nonfinite(&self, v: &Number) -> String {
        let s = float_special(&v.re);
        if !v.is_complex() || v.im.is_zero() {
            return s;
        }
        // Non-finite imaginary component — not covered by the catalog;
        // rendered with the complex join rules.
        format!("{s}+{}i", float_special(&v.im))
    }

    fn serialize_complex(&self, v: &Number) -> Result<String, Error> {
        // render_real ignores the imaginary field, so v itself renders the
        // real part (the fixed-format cap sees both parts).
        let (re_s, _) = self.render_real(v)?;
        let im_neg = v.im < 0;
        let im_mag = Number::float(v.im.clone().abs());
        let (im_s, _) = self.render_real(&im_mag)?;
        let zero_s = self.zero_string();
        let one_s = self.one_string();
        if re_s == zero_s && im_s == zero_s {
            return Ok(zero_s);
        }
        if im_s == zero_s {
            return Ok(re_s);
        }
        // Imaginary 1 renders as a bare `i` (doc 07 §2.6).
        let mag = if im_s == one_s {
            "i".to_string()
        } else {
            format!("{im_s}i")
        };
        if re_s == zero_s {
            return Ok(if im_neg { format!("{MINUS}{mag}") } else { mag });
        }
        Ok(format!("{re_s}{}{mag}", if im_neg { MINUS } else { '+' }))
    }

    // ------------------------------------------------------------------
    // Exact digit extraction
    // ------------------------------------------------------------------

    /// Nonnegative exact magnitude of a real value; `None` for zero.
    fn source(v: &Number) -> Option<Src> {
        if v.re.is_zero() {
            return None;
        }
        if let Some(r) = v.exact() {
            let p = r.numer();
            if p.is_zero() {
                return None;
            }
            let p = if *p < 0 { -p.clone() } else { p.clone() };
            return Some(Src::Rat(p, r.denom().clone()));
        }
        let (m, e) = v.re.to_integer_exp()?;
        let m = if m < 0 { -m } else { m };
        Some(Src::Bin(m, e))
    }

    /// `floor(|v| · b^k)` for any i64 k, computed on integers only.
    ///
    /// * `p/q`: scale the fraction directly.
    /// * `m·2^e` with `b = odd·2^s`: `|v|·b^k = m·odd^k·2^(e+s·k)`; when the
    ///   odd part divides (k < 0) the shift is applied before the division,
    ///   or after it when both are shifts (nested floor division is exact).
    fn trunc_scaled(&self, src: &Src, k: i64) -> Integer {
        let b = self.repr_base;
        match src {
            Src::Rat(p, q) => {
                if k >= 0 {
                    let scale = Integer::from(b).pow(k as u32);
                    p.clone() * scale / q.clone()
                } else {
                    let scale = Integer::from(b).pow((-k) as u32);
                    p.clone() / (q.clone() * scale)
                }
            }
            Src::Bin(m, e) => {
                let s = b.trailing_zeros() as i64;
                let odd = b >> b.trailing_zeros();
                let sft: i64 = *e as i64 + s * k;
                if k >= 0 {
                    let t = if odd == 1 {
                        m.clone()
                    } else {
                        m.clone() * Integer::from(odd).pow(k as u32)
                    };
                    if sft >= 0 {
                        t << (sft as u32)
                    } else {
                        t >> ((-sft) as u32)
                    }
                } else {
                    let den = if odd == 1 {
                        Integer::from(1)
                    } else {
                        Integer::from(odd).pow((-k) as u32)
                    };
                    if sft >= 0 {
                        (m.clone() << (sft as u32)) / den
                    } else {
                        let t = m.clone() / den;
                        t >> ((-sft) as u32)
                    }
                }
            }
        }
    }

    /// `round_half_away(|v| · mult)` for an arbitrary positive integer
    /// multiplier — exact for both sources (the DMS total-seconds scale).
    fn round_scaled(&self, src: &Src, mult: &Integer) -> Integer {
        match src {
            Src::Rat(p, q) => {
                (Integer::from(2) * p.clone() * mult.clone() + q.clone())
                    / (Integer::from(2) * q.clone())
            }
            Src::Bin(m, e) => {
                let mut odd_m = mult.clone();
                let mut s_m = 0i64;
                while (odd_m.clone() & Integer::from(1)).is_zero() {
                    odd_m >>= 1;
                    s_m += 1;
                }
                let n = m.clone() * odd_m;
                let sft = *e as i64 + s_m;
                if sft >= 0 {
                    n << (sft as u32)
                } else {
                    let t = (-sft) as u32;
                    (n + (Integer::from(1) << (t - 1))) >> t
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Digit rendering helpers
    // ------------------------------------------------------------------

    /// Digits of a nonnegative integer in the representation base.
    fn digit_string(&self, n: &Integer) -> String {
        if n.is_zero() {
            return "0".to_string();
        }
        let bi = Integer::from(self.repr_base);
        let mut n = n.clone();
        let mut chars = Vec::new();
        while !n.is_zero() {
            let d = n.clone() % bi.clone();
            chars.push(DIGITS[d.to_u32().unwrap_or(0) as usize] as char);
            n /= bi.clone();
        }
        chars.into_iter().rev().collect()
    }

    /// Exactly `a` fraction digits (zero padded), trailing zeros stripped
    /// unless `show_zeroes` (doc 07 §2.1, §3).
    fn frac_string(&self, frac: &Integer, a: u32) -> String {
        if a == 0 {
            return String::new();
        }
        let bi = Integer::from(self.repr_base);
        let mut n = frac.clone();
        let mut chars = Vec::with_capacity(a as usize);
        for _ in 0..a {
            let d = n.clone() % bi.clone();
            chars.push(DIGITS[d.to_u32().unwrap_or(0) as usize] as char);
            n /= bi.clone();
        }
        chars.reverse();
        let mut s: String = chars.into_iter().collect();
        if !self.show_zeroes {
            while s.ends_with('0') {
                s.pop();
            }
        }
        s
    }

    /// Grouping: every 3 digits in decimal with the locale separator, every
    /// 4 digits with a narrow space in other bases; disabled when the locale
    /// separator equals the radix (doc 07 §1).
    fn group(&self, digits: &str) -> String {
        let (g, sep): (usize, String) = if self.repr_base == 10 {
            if self.thousands == self.radix {
                return digits.to_string();
            }
            (3, self.thousands.to_string())
        } else {
            (4, NARROW_SPACE.to_string())
        };
        if !self.show_thousands {
            return digits.to_string();
        }
        let n = digits.chars().count();
        let mut out = String::with_capacity(digits.len() + n / g * sep.len());
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (n - i).is_multiple_of(g) {
                out.push_str(&sep);
            }
            out.push(ch);
        }
        out
    }

    /// Exponent as superscript digits in the current base, with a
    /// superscript minus for negative exponents (doc 07 §2.3, §2.5).
    fn superscript_exponent(&self, e: i64) -> String {
        let mut out = String::new();
        if e < 0 {
            out.push(SUP_MINUS);
        }
        let b = self.repr_base as u64;
        let mut n = e.unsigned_abs();
        let mut digits = Vec::new();
        loop {
            digits.push((n % b) as u32);
            n /= b;
            if n == 0 {
                break;
            }
        }
        for d in digits.into_iter().rev() {
            out.push(if d < 10 {
                SUP[d as usize]
            } else {
                SUP_LETTERS[d as usize - 10]
            });
        }
        out
    }

    /// Subscript base suffix digits (`₂`, `₁₆`) — always decimal digits.
    fn base_suffix(&self) -> String {
        let mut n = self.repr_base;
        let mut digits = Vec::new();
        while n > 0 {
            digits.push((n % 10) as usize);
            n /= 10;
        }
        digits.into_iter().rev().map(|d| SUB[d]).collect()
    }

    /// The formatted zero (doc 07 §3): `0`, or padded to the accuracy when
    /// `show-zeroes` is on. Used for complex part suppression.
    fn zero_string(&self) -> String {
        self.const_string('0')
    }

    fn one_string(&self) -> String {
        self.const_string('1')
    }

    fn const_string(&self, c: char) -> String {
        if self.show_zeroes && self.accuracy > 0 {
            format!("{c}{}{}", self.radix, "0".repeat(self.accuracy as usize))
        } else {
            c.to_string()
        }
    }
}

/// `∞`, `−∞` or `NaN` for a non-finite float part (FMT-T017).
fn float_special(f: &Float) -> String {
    if f.is_nan() {
        "NaN".to_string()
    } else if *f < 0 {
        format!("{MINUS}∞")
    } else {
        "∞".to_string()
    }
}

/// Exact rational value of a plain decimal literal (test helper).
#[allow(dead_code)]
pub fn rational_from_decimal(s: &str) -> Rational {
    crate::number::parse_decimal_rational(s).expect("valid decimal literal")
}
