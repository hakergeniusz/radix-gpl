//! Numeric engine (spec: doc 03).
//!
//! A [`Number`] is a complex value: real and imaginary parts are
//! arbitrary-precision reals at the working precision `P` (round to nearest,
//! doc 03 §2), plus:
//!
//! * an **exact rational shadow** carried while the value is exactly `p/q`
//!   (doc 03 §1 — the v51 "keep rational arithmetic exact" behavior), and
//! * an optional **attached error** for the "value 0 displayed with error"
//!   cases such as `0^0` (doc 03 §9).
//!
//! Arithmetic between two shadowed real values is performed on the exact
//! rationals; the MPFR parts are kept rounded for display. Any operation that
//! can be inexact drops the shadow.

use crate::error::{messages, Error};
use crate::numfuncs::pi_const;
use rug::ops::Pow;
use rug::{Float, Integer, Rational};

/// A calculator value: complex MPFR pair + exact rational shadow + error flag.
#[derive(Debug, Clone)]
pub struct Number {
    pub re: Float,
    pub im: Float,
    /// Exact rational value while the number is a real that is exactly `p/q`.
    exact: Option<Rational>,
    /// Attached error: the value is still usable (e.g. `0^0` → 0 with error).
    pub err: Option<Error>,
}

impl Number {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// Zero at the given working precision.
    pub fn zero(prec: u32) -> Self {
        Number {
            re: Float::new(prec),
            im: Float::new(prec),
            exact: Some(Rational::new()),
            err: None,
        }
    }

    /// One at the given working precision.
    pub fn one(prec: u32) -> Self {
        Self::rational(Rational::from(1), prec)
    }

    /// Exact rational value (shadow carried).
    pub fn rational(r: Rational, prec: u32) -> Self {
        let f = Float::with_val(prec, &r);
        Number {
            re: f,
            im: Float::new(prec),
            exact: Some(r),
            err: None,
        }
    }

    /// Exact integer value (shadow carried).
    pub fn integer(i: Integer, prec: u32) -> Self {
        Self::rational(Rational::from(i), prec)
    }

    /// Small integer constant.
    pub fn int(i: i64, prec: u32) -> Self {
        Self::rational(Rational::from(i), prec)
    }

    /// Value from an `f64` (exact binary rational; keeps inf/NaN bit patterns,
    /// doc 03 §1). Normalizes negative zero.
    pub fn from_f64(v: f64, prec: u32) -> Self {
        let re = Float::with_val(prec, v);
        if v.is_finite() {
            let r = double_to_rational(v);
            Number {
                re,
                im: Float::new(prec),
                exact: Some(r),
                err: None,
            }
        } else {
            Number {
                re,
                im: Float::new(prec),
                exact: None,
                err: None,
            }
        }
    }

    /// Complex value from two floats (shadow dropped). Parts are normalized
    /// (−0 → +0).
    pub fn complex(re: Float, im: Float) -> Self {
        let mut n = Number {
            re,
            im,
            exact: None,
            err: None,
        };
        n.normalize_zeros();
        n
    }

    /// Real value from a float (shadow dropped, −0 normalized).
    pub fn float(re: Float) -> Self {
        let prec = re.prec();
        Self::complex(re, Float::new(prec))
    }

    /// Zero with an attached error (doc 03 §9: `0^0` returns 0 with error).
    pub fn zero_with_error(prec: u32, err: Error) -> Self {
        let mut n = Self::zero(prec);
        n.err = Some(err);
        n
    }

    /// This value with an attached status error (doc 03 §9: value displayed
    /// with error, e.g. Ddb's negative-period status).
    pub fn with_error(mut self, err: Error) -> Self {
        self.err = Some(err);
        self
    }

    /// π at the given precision (no shadow — irrational).
    pub fn pi(prec: u32) -> Self {
        Self::float(pi_const(prec))
    }

    /// Working precision of this value in bits.
    pub fn prec(&self) -> u32 {
        self.re.prec()
    }

    // ------------------------------------------------------------------
    // Predicates (doc 03 §1, §8)
    // ------------------------------------------------------------------

    /// True when both parts are zero.
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }

    /// True when the imaginary part is nonzero.
    pub fn is_complex(&self) -> bool {
        !self.im.is_zero()
    }

    /// True when the imaginary part is zero.
    pub fn is_real(&self) -> bool {
        self.im.is_zero()
    }

    /// True when the value is a real integer.
    pub fn is_integer(&self) -> bool {
        if let Some(r) = &self.exact {
            r.is_integer()
        } else {
            self.is_real() && self.re.is_integer()
        }
    }

    /// True when the value is a non-negative integer (doc 02 §7 `is_natural`).
    pub fn is_natural(&self) -> bool {
        self.is_integer() && self.re >= 0
    }

    /// True when the value is finite (not inf/NaN).
    pub fn is_finite(&self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// Exact rational shadow, if carried.
    pub fn exact(&self) -> Option<&Rational> {
        self.exact.as_ref()
    }

    /// The exact integer value of an integer Number.
    pub fn to_integer(&self) -> Option<Integer> {
        if self.is_complex() {
            return None;
        }
        if let Some(r) = &self.exact {
            if r.is_integer() {
                return Some(rat_trunc(r));
            }
            return None;
        }
        if self.re.is_finite() {
            return self.re.to_integer();
        }
        None
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn normalize_zeros(&mut self) {
        if self.re.is_zero() {
            self.re = Float::new(self.prec());
        }
        if self.im.is_zero() {
            self.im = Float::new(self.prec());
        }
    }

    /// Result envelope for binary ops: precision and error propagation.
    fn result_envelope(a: &Number, b: &Number) -> (u32, Option<Error>) {
        let prec = a.prec().max(b.prec());
        let err = match (&a.err, &b.err) {
            (Some(e), _) => Some(e.clone()),
            (None, e) => e.clone(),
        };
        (prec, err)
    }

    /// Set the exact shadow and re-round the MPFR parts from it.
    pub(crate) fn finish_exact(mut self, r: Rational) -> Self {
        let prec = self.prec();
        self.re = Float::with_val(prec, &r);
        self.im = Float::new(prec);
        self.exact = Some(r);
        self
    }

    // ------------------------------------------------------------------
    // Arithmetic (doc 03 §4.1)
    // ------------------------------------------------------------------

    /// `+`: exact on two shadowed reals, else MPFR at `P` bits.
    pub fn add(&self, other: &Number) -> Number {
        let (prec, err) = Self::result_envelope(self, other);
        if self.is_real() && other.is_real() {
            if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
                return Number::zero(prec).finish_exact(a.clone() + b.clone());
            }
        }
        let re = self.re.clone() + other.re.clone();
        let im = self.im.clone() + other.im.clone();
        let mut out = Number {
            re,
            im,
            exact: None,
            err,
        };
        out.normalize_zeros();
        out
    }

    /// `−`: exact on two shadowed reals.
    pub fn sub(&self, other: &Number) -> Number {
        let (prec, err) = Self::result_envelope(self, other);
        if self.is_real() && other.is_real() {
            if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
                return Number::zero(prec).finish_exact(a.clone() - b.clone());
            }
        }
        let re = self.re.clone() - other.re.clone();
        let im = self.im.clone() - other.im.clone();
        let mut out = Number {
            re,
            im,
            exact: None,
            err,
        };
        out.normalize_zeros();
        out
    }

    /// `×`: exact on two shadowed reals; complex via components.
    pub fn mul(&self, other: &Number) -> Number {
        let (prec, err) = Self::result_envelope(self, other);
        if self.is_real() && other.is_real() {
            if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
                return Number::zero(prec).finish_exact(a.clone() * b.clone());
            }
        }
        let re = self.re.clone() * other.re.clone() - self.im.clone() * other.im.clone();
        let im = self.re.clone() * other.im.clone() + self.im.clone() * other.re.clone();
        let mut out = Number {
            re,
            im,
            exact: None,
            err,
        };
        out.normalize_zeros();
        out
    }

    /// `÷`: division by zero (real or complex) is an error (doc 03 §4.1);
    /// exact on two shadowed reals.
    pub fn div(&self, other: &Number) -> Result<Number, Error> {
        if other.is_zero() {
            return Err(Error::mp(messages::DIV_BY_ZERO));
        }
        let (prec, err) = Self::result_envelope(self, other);
        if self.is_real() && other.is_real() {
            if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
                return Ok(Number::zero(prec).finish_exact(a.clone() / b.clone()));
            }
            // Real division: imaginary component exactly +0 (doc 03 §3).
            let re = self.re.clone() / other.re.clone();
            return Ok(Number {
                re,
                im: Float::new(prec),
                exact: None,
                err,
            });
        }
        // (ac+bd)/(c²+d²), (bc−ad)/(c²+d²).
        let den = other.re.clone() * other.re.clone() + other.im.clone() * other.im.clone();
        let ac = self.re.clone() * other.re.clone();
        let bd = self.im.clone() * other.im.clone();
        let bc = self.im.clone() * other.re.clone();
        let ad = self.re.clone() * other.im.clone();
        let re = (ac + bd) / den.clone();
        let im = (bc - ad) / den;
        let mut out = Number {
            re,
            im,
            exact: None,
            err,
        };
        out.normalize_zeros();
        Ok(out)
    }

    /// Negation (exact on shadowed values).
    pub fn neg(&self) -> Number {
        let mut out = Number {
            re: -self.re.clone(),
            im: -self.im.clone(),
            exact: self.exact.as_ref().map(|r| -r.clone()),
            err: self.err.clone(),
        };
        out.normalize_zeros();
        out
    }

    /// Postfix percent: `x%` = x/100 (doc 02 §5.1). Exact on shadows.
    pub fn percent(&self) -> Result<Number, Error> {
        let hundred = Number::int(100, self.prec());
        self.div(&hundred)
    }

    /// `mod` (doc 03 §4.1): `z = x − floor(x/y)·y`, then if y<0 and z>0, or
    /// y>0 and z<0, then `z += y`. Real numbers only.
    pub fn mod_op(&self, other: &Number) -> Result<Number, Error> {
        if self.is_complex() || other.is_complex() {
            return Err(Error::mp(messages::MOD_COMPLEX));
        }
        if other.is_zero() {
            return Err(Error::mp(messages::DIV_BY_ZERO));
        }
        let (prec, err) = Self::result_envelope(self, other);
        if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
            let q = rat_floor(&(a.clone() / b.clone()));
            let mut z = a.clone() - Rational::from(q) * b.clone();
            let b_neg = *b < 0;
            if (b_neg && z > 0) || (!b_neg && z < 0) {
                z += b.clone();
            }
            return Ok(Number::zero(prec).finish_exact(z));
        }
        let x = &self.re;
        let y = &other.re;
        let q = (x.clone() / y.clone()).floor();
        let mut z = x.clone() - q * y.clone();
        let y_neg = *y < 0;
        if (y_neg && z > 0) || (!y_neg && z < 0) {
            z += y.clone();
        }
        Ok(Number {
            re: z,
            im: Float::new(prec),
            exact: None,
            err,
        })
    }

    // ------------------------------------------------------------------
    // Comparison helpers (doc 03 §8)
    // ------------------------------------------------------------------

    /// Equality (exact when both carry shadows).
    pub fn equals(&self, other: &Number) -> bool {
        if self.is_complex() || other.is_complex() {
            return self.re == other.re && self.im == other.im;
        }
        if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
            return a == b;
        }
        self.re == other.re
    }

    /// Three-way comparison of real values. Complex values compare their
    /// real parts (internal use only).
    pub fn cmp_value(&self, other: &Number) -> std::cmp::Ordering {
        if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
            if self.is_real() && other.is_real() {
                return a.cmp(b);
            }
        }
        self.re
            .partial_cmp(&other.re)
            .unwrap_or(std::cmp::Ordering::Equal)
    }

    /// Kronecker delta `cmp(x; y)`: 1 if equal else 0 (doc 04 §1.6).
    pub fn cmp_delta(&self, other: &Number) -> Number {
        let v = if self.equals(other) { 1 } else { 0 };
        let mut n = Self::int(v, self.prec().max(other.prec()));
        n.err = match (&self.err, &other.err) {
            (Some(e), _) => Some(e.clone()),
            (None, e) => e.clone(),
        };
        n
    }

    /// Magnitude test used by the serializer's fixed-format cap: true when
    /// |value| ≥ 2^`exp2` (doc 07 §2.1). Complex values use max(|re|, |im|).
    pub fn magnitude_geq_power_of_two(&self, exp2: u32) -> bool {
        let limit = Float::with_val(self.prec() + 8, Integer::from(1) << exp2);
        let a = self.re.clone().abs();
        let m = if self.is_complex() {
            let b = self.im.clone().abs();
            if a > b {
                a
            } else {
                b
            }
        } else {
            a
        };
        let m = Float::with_val(self.prec() + 8, m);
        m >= limit
    }

    // ------------------------------------------------------------------
    // Absolute value / sign / complex accessors
    // ------------------------------------------------------------------

    /// `abs` — modulus for complex values (doc 03 §4.7).
    pub fn abs(&self) -> Number {
        if self.is_complex() {
            let m = self.re.clone().hypot(&self.im);
            return Self::float(m);
        }
        if let Some(r) = &self.exact {
            let pos = if *r < 0 { -r.clone() } else { r.clone() };
            return Number::zero(self.prec()).finish_exact(pos);
        }
        let a = self.re.clone().abs();
        let mut n = Self::float(a);
        n.err = self.err.clone();
        n
    }

    /// `sgn`: −1/0/1 for reals; x/|x| for complex (doc 03 §4.7).
    pub fn sgn(&self) -> Result<Number, Error> {
        if self.is_complex() {
            let m = self.re.clone().hypot(&self.im);
            let re = self.re.clone() / m.clone();
            let im = self.im.clone() / m;
            let mut n = Self::complex(re, im);
            n.err = self.err.clone();
            return Ok(n);
        }
        if self.is_zero() {
            let mut n = Self::zero(self.prec());
            n.err = self.err.clone();
            return Ok(n);
        }
        let v = if self.re < 0 { -1 } else { 1 };
        let mut n = Self::int(v, self.prec());
        n.err = self.err.clone();
        Ok(n)
    }

    /// Complex conjugate.
    pub fn conj(&self) -> Number {
        let mut out = self.clone();
        out.im = -self.im.clone();
        if self.is_complex() {
            out.exact = None;
        }
        out.normalize_zeros();
        out
    }

    /// Real component.
    pub fn re_part(&self) -> Number {
        let exact = if self.is_real() {
            self.exact.clone()
        } else {
            None
        };
        Number {
            re: self.re.clone(),
            im: Float::new(self.prec()),
            exact,
            err: self.err.clone(),
        }
    }

    /// Imaginary component.
    pub fn im_part(&self) -> Number {
        Number {
            re: self.im.clone(),
            im: Float::new(self.prec()),
            exact: None,
            err: self.err.clone(),
        }
    }
}

// ----------------------------------------------------------------------
// Free helpers
// ----------------------------------------------------------------------

/// Exact rational value of a finite double.
pub fn double_to_rational(v: f64) -> Rational {
    let bits = v.to_bits();
    let mantissa_field = bits & 0x000F_FFFF_FFFF_FFFF;
    let raw_exp = ((bits >> 52) & 0x7FF) as i64;
    let (mut mantissa, mut exp) = if raw_exp == 0 {
        // Zero or subnormal: no implicit leading bit, exponent −1074.
        (Integer::from(mantissa_field), -1074i64)
    } else {
        (
            Integer::from(mantissa_field | 0x0010_0000_0000_0000),
            raw_exp - 1075,
        )
    };
    if mantissa.is_zero() {
        return Rational::from(0);
    }
    if exp > 0 {
        mantissa <<= exp as u32;
        exp = 0;
    }
    let den = Integer::from(1) << (-exp) as u32;
    let r = Rational::from(mantissa) / Rational::from(den);
    if v < 0.0 {
        -r
    } else {
        r
    }
}

/// Exact parse of a plain decimal literal string (`[sign]digits[.digits]`)
/// into a Rational (e.g. `0.1` → 1/10). The expression lexer uses the digit
/// form; this is for decimal literals with a radix point.
pub fn parse_decimal_rational(s: &str) -> Option<Rational> {
    let (sign, body) = match s.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((a, b)) => (a, b),
        None => (body, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{int_part}{frac_part}");
    let num: Integer = digits.parse().ok()?;
    let mut r = Rational::from(num);
    if !frac_part.is_empty() {
        r /= Rational::from(Integer::from(10).pow(frac_part.len() as u32));
    }
    if sign < 0 {
        r = -r;
    }
    Some(r)
}

/// |r| for rationals.
pub fn abs_rat(r: &Rational) -> Rational {
    if *r < 0 {
        -r.clone()
    } else {
        r.clone()
    }
}

/// Truncation of a rational toward zero as an Integer (canonical form has a
/// positive denominator, so this is plain truncating division).
pub fn rat_trunc(r: &Rational) -> Integer {
    r.numer().clone() / r.denom().clone()
}

/// floor of a rational as Integer.
pub fn rat_floor(r: &Rational) -> Integer {
    let q = rat_trunc(r);
    if *r < 0 && !r.is_integer() {
        q - Integer::from(1)
    } else {
        q
    }
}

/// ceiling of a rational as Integer.
pub fn rat_ceil(r: &Rational) -> Integer {
    let q = rat_trunc(r);
    if *r > 0 && !r.is_integer() {
        q + Integer::from(1)
    } else {
        q
    }
}

/// floor-modulus on Integers (result has the divisor's sign).
pub fn int_mod_floor(a: &Integer, m: &Integer) -> Integer {
    let r = a.clone() % m.clone();
    if !r.is_zero() && ((r < 0) != (*m < 0)) {
        r + m.clone()
    } else {
        r
    }
}

/// Uniform random value in [0, 1) as a Number with an exact binary-rational
/// shadow (doc 03 §7: fresh value per read).
pub fn random_unit(prec: u32) -> Number {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0x2545_F491_4F6C_DD1D);
    let mut s = nanos | 1;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    // 53 random bits → uniform double in [0,1).
    let hi = next() & ((1 << 21) - 1);
    let lo = next() & ((1 << 32) - 1);
    let v = ((hi << 32) | lo) as f64 / (1u64 << 53) as f64;
    Number::from_f64(v, prec)
}
