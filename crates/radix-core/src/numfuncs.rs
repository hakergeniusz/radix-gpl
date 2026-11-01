//! Mathematical functions on [`Number`] (spec: doc 03 §4, doc 04 §1).
//!
//! Angle-unit aware trig uses the documented exactness guards (doc 03 §4.4):
//! axis angles return exactly 0 and undefined tangents error before any
//! floating-point work.

use crate::error::{messages, Error, ErrorCode};
use crate::number::{abs_rat, int_mod_floor, rat_floor, Number};
use rug::ops::Pow;
use rug::{Complex, Float, Integer, Rational};

/// Angle unit (doc 03 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngleUnit {
    Radians,
    Degrees,
    Gradians,
}

/// π at a given precision.
pub fn pi_const(prec: u32) -> Float {
    Float::with_val(prec, rug::float::Constant::Pi)
}

// ----------------------------------------------------------------------
// Powers and roots (doc 03 §4.2)
// ----------------------------------------------------------------------

impl Number {
    /// `x^y` (doc 03 §4.2). Reciprocal-integer exponents take the root path;
    /// integer exponents of shadowed rationals stay exact; the general case
    /// uses complex power.
    pub fn pow_op(&self, y: &Number) -> Result<Number, Error> {
        // 0^y cases (doc 03 §9: value 0 with error).
        if self.is_zero() {
            if y.is_zero() {
                return Ok(Number::zero_with_error(
                    self.prec(),
                    Error::mp(messages::ZERO_POW_ZERO),
                ));
            }
            if y.is_real() && y.re < 0 {
                return Ok(Number::zero_with_error(
                    self.prec(),
                    Error::mp(messages::ZERO_POW_NEG),
                ));
            }
            return Ok(self.clone());
        }
        if y.is_zero() {
            return Ok(Number::one(self.prec()));
        }
        if y.equals(&Number::one(y.prec())) {
            return Ok(self.clone());
        }
        // Reciprocal-integer exponent → root path (doc 03 §4.2).
        if y.is_real() && !y.is_integer() {
            if let Some(n) = reciprocal_integer(y) {
                return self.root(n);
            }
        }
        // Integer exponents use the dedicated integer-power code (doc 03
        // §4.2): exact for shadows (mul keeps them) and component-exact for
        // complex values (i² = −1). Bounded to keep intermediate integers
        // reasonable; larger exponents fall through to complex power.
        if y.is_real() && y.is_integer() {
            if let Some(n) = y.to_integer().and_then(|n| n.to_i64()) {
                let bits = self
                    .re
                    .clone()
                    .abs()
                    .to_integer()
                    .map(|i| i.significant_bits())
                    .unwrap_or(0);
                if u128::from(n.unsigned_abs()) * u128::from(bits.max(1)) <= 8_000_000 {
                    return self.pow_integer_exact(n);
                }
            }
        }
        // General complex power.
        let prec = self.prec().max(y.prec());
        let base = Complex::with_val(prec, (&self.re, &self.im));
        let exp = Complex::with_val(prec, (&y.re, &y.im));
        let r = Pow::pow(base, exp);
        let err = self.err.clone().or_else(|| y.err.clone());
        let out = Number::complex(r.real().clone(), r.imag().clone());
        let out = check_mpfr_range(&out)?;
        let mut out = out;
        out.err = err;
        Ok(out)
    }

    /// Dedicated integer-power code (doc 03 §4.2): repeated squaring via
    /// component arithmetic; shadows stay exact, complex values stay
    /// component-exact.
    pub fn pow_integer_exact(&self, n: i64) -> Result<Number, Error> {
        let mut acc = Number::one(self.prec());
        let mut base = self.clone();
        let mut e = n.unsigned_abs();
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        if n < 0 {
            return Number::one(self.prec()).div(&acc);
        }
        acc.err = acc.err.clone().or_else(|| self.err.clone());
        Ok(acc)
    }

    /// Superscript-power entry point routed through the general power.
    pub fn pow_integer(&self, n: i64) -> Result<Number, Error> {
        self.pow_op(&Number::int(n, self.prec()))
    }

    /// `root(n)` (doc 03 §4.2): n=0 errors; n<0 inverts; negative base with
    /// odd n is a real root, with even n the principal complex root.
    pub fn root(&self, n: i64) -> Result<Number, Error> {
        if n == 0 {
            return Err(Error::mp(messages::ZEROTH_ROOT));
        }
        if n < 0 {
            let inv = Number::one(self.prec()).div(self)?;
            return inv.root(-n);
        }
        let n = n as u32;
        // Exact rational root when it exists.
        if self.is_real() {
            if let Some(x) = self.exact().cloned() {
                if let Some(r) = rational_root_exact(&x, n) {
                    let mut out = Number::zero(self.prec());
                    out.err = self.err.clone();
                    return Ok(out.finish_exact(r));
                }
            }
        }
        if self.is_complex() {
            let prec = self.prec();
            let z = Complex::with_val(prec, (&self.re, &self.im));
            let e = Complex::with_val(prec, Float::with_val(prec, 1) / n);
            let r = Pow::pow(z, e);
            let mut out = Number::complex(r.real().clone(), r.imag().clone());
            out.err = self.err.clone();
            return Ok(out);
        }
        if self.re >= 0 {
            let r = self.re.clone().root(n);
            let mut out = Number::float(r);
            out.err = self.err.clone();
            Ok(out)
        } else if n % 2 == 1 {
            // Odd root of a negative real is real.
            let m = self.re.clone().abs();
            let r = m.root(n);
            let mut out = Number::float(-r);
            out.err = self.err.clone();
            Ok(out)
        } else if n == 2 {
            // √ of a negative real: exactly 0 + √|x|·i (doc 03 §4.2).
            let m = self.re.clone().abs();
            let im = m.root(2);
            let mut out = Number::complex(Float::new(self.prec()), im);
            out.err = self.err.clone();
            Ok(out)
        } else {
            // Principal complex root: r·(cos(π/n) + i·sin(π/n)).
            let prec = self.prec();
            let m = self.re.clone().abs();
            let r = m.root(n);
            let pi = pi_const(prec);
            let ang = pi / n;
            let (c, s) = (ang.clone().cos(), ang.sin());
            let re = r.clone() * c;
            let im = r * s;
            let mut out = Number::complex(re, im);
            out.err = self.err.clone();
            Ok(out)
        }
    }
}

/// If y = 1/n for an integer n ≥ 2 (checked exactly), return n.
fn reciprocal_integer(y: &Number) -> Option<i64> {
    if let Some(r) = y.exact() {
        // r = p/q reduced: reciprocal integer iff |p| == 1.
        let (num, den) = (r.numer(), r.denom());
        if num.clone().abs().to_u64() == Some(1) {
            let n = den.to_i64()?;
            let n = if *num < 0 { -n } else { n };
            if n >= 2 {
                return Some(n);
            }
        }
        return None;
    }
    // Float fallback: 1/y integer and y == 1/n exactly.
    if !y.is_real() || !y.re.is_finite() || y.re <= 0 {
        return None;
    }
    let n = y.re.clone().recip().to_integer()?;
    let nf = Float::with_val(y.prec() + 8, &n);
    if nf >= 2 && nf * y.re.clone() == 1 {
        n.to_i64().filter(|&v| v >= 2)
    } else {
        None
    }
}

/// Exact n-th root of a rational when numerator and denominator are perfect
/// n-th powers.
fn rational_root_exact(x: &Rational, n: u32) -> Option<Rational> {
    let (num, den) = (x.numer(), x.denom());
    let neg = *num < 0;
    if neg && n.is_multiple_of(2) {
        return None;
    }
    let (rootn, exactn) = integer_root_exact(num.clone().abs(), n);
    let (rootd, exactd) = integer_root_exact(den.clone(), n);
    if !(exactn && exactd) {
        return None;
    }
    let rootn = if neg { -rootn } else { rootn };
    Some(Rational::from(rootn) / Rational::from(rootd))
}

/// Integer n-th root: (root, exact?) via mpz root/remainder.
fn integer_root_exact(x: Integer, n: u32) -> (Integer, bool) {
    if x.to_u64() == Some(0) {
        return (Integer::from(0), true);
    }
    let (root, rem) = x.root_rem(Integer::from(0), n);
    (root, rem.is_zero())
}

// ----------------------------------------------------------------------
// Exponentials and logarithms (doc 03 §4.3)
// ----------------------------------------------------------------------

impl Number {
    /// e^x over the complex field.
    pub fn exp(&self) -> Result<Number, Error> {
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.exp();
        let out = Number::complex(r.real().clone(), r.imag().clone());
        check_mpfr_range(&out)
    }

    /// Natural logarithm; ln(x<0) = ln|x| + πi (doc 03 §4.3).
    pub fn ln(&self) -> Result<Number, Error> {
        if self.is_zero() {
            return Err(Error::mp(messages::LOG_ZERO));
        }
        if self.is_real() {
            if self.re > 0 {
                let mut out = Number::float(self.re.clone().ln());
                out.err = self.err.clone();
                return Ok(out);
            }
            // Negative real: ln|x| + πi with the +π branch (doc 03 §4.3).
            let m = self.re.clone().abs();
            let re = m.ln();
            let im = pi_const(self.prec());
            let mut out = Number::complex(re, im);
            out.err = self.err.clone();
            return Ok(out);
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.ln();
        let mut out = Number::complex(r.real().clone(), r.imag().clone());
        // MPC's branch for negative-real inputs must yield +πi.
        if self.re < 0 && self.im.is_zero() && out.im < 0 {
            out.im = pi_const(prec);
        }
        out.err = self.err.clone();
        Ok(out)
    }

    /// log₁₀(x), or log_y(x) with a base (doc 03 §4.3, doc 04 §1.6).
    pub fn log(&self, base: Option<&Number>) -> Result<Number, Error> {
        match base {
            None => self.log10(),
            Some(b) => {
                if b.is_zero() || b.equals(&Number::one(b.prec())) {
                    return Err(Error::mp(messages::LOG_BASE));
                }
                let lx = self.ln()?;
                let lb = b.ln()?;
                lx.div(&lb)
            }
        }
    }

    fn log10(&self) -> Result<Number, Error> {
        if self.is_zero() {
            return Err(Error::mp(messages::LOG_ZERO));
        }
        if self.is_real() {
            if self.re > 0 {
                let mut out = Number::float(self.re.clone().log10());
                out.err = self.err.clone();
                return Ok(out);
            }
            // log(−x) = log₁₀(x) + πi/ln(10) (log(−10) − (1+πi/ln 10) = 0).
            let m = self.re.clone().abs();
            let re = m.log10();
            let im = pi_const(self.prec()) / Float::with_val(self.prec(), 10).ln();
            let mut out = Number::complex(re, im);
            out.err = self.err.clone();
            return Ok(out);
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.log10();
        let mut out = Number::complex(r.real().clone(), r.imag().clone());
        if self.re < 0 && self.im.is_zero() && out.im < 0 {
            out.im = pi_const(prec) / Float::with_val(prec, 10).ln();
        }
        out.err = self.err.clone();
        Ok(out)
    }
}

// ----------------------------------------------------------------------
// Trigonometry (doc 03 §4.4)
// ----------------------------------------------------------------------

impl Number {
    /// sin with angle-unit exactness guards.
    pub fn sin(&self, unit: AngleUnit) -> Result<Number, Error> {
        if let Some(zero) = self.trig_axis_zero(unit, AxisKind::SinZero)? {
            return Ok(zero);
        }
        let arg = self.to_radians_angle(unit);
        self.complex_trig(|z| z.sin(), arg)
    }

    /// cos with angle-unit exactness guards.
    pub fn cos(&self, unit: AngleUnit) -> Result<Number, Error> {
        if let Some(zero) = self.trig_axis_zero(unit, AxisKind::CosZero)? {
            return Ok(zero);
        }
        let arg = self.to_radians_angle(unit);
        self.complex_trig(|z| z.cos(), arg)
    }

    /// tan: undefined at 90° + 180°·n (in the active unit); exact 0 at
    /// multiples of the half-turn; otherwise complex tan.
    pub fn tan(&self, unit: AngleUnit) -> Result<Number, Error> {
        if let Some(zero) = self.trig_axis_zero(unit, AxisKind::Tan)? {
            return Ok(zero);
        }
        let arg = self.to_radians_angle(unit);
        self.complex_trig(|z| z.tan(), arg)
    }

    /// Inverse sine (doc 03 §4.4): real domain [−1, 1]; result in the active
    /// angle unit.
    pub fn asin(&self, unit: AngleUnit) -> Result<Number, Error> {
        if self.is_real() && self.is_finite() && self.re.clone().abs() > 1 {
            return Err(Error::mp(messages::ASIN_DOMAIN));
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.asin();
        let out = Number::float(r.real().clone());
        out.apply_angle_unit(unit)
    }

    /// Inverse cosine.
    pub fn acos(&self, unit: AngleUnit) -> Result<Number, Error> {
        if self.is_real() && self.is_finite() && self.re.clone().abs() > 1 {
            return Err(Error::mp(messages::ACOS_DOMAIN));
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.acos();
        let out = Number::float(r.real().clone());
        out.apply_angle_unit(unit)
    }

    /// Inverse tangent; ±i is undefined (doc 04 §1.1).
    pub fn atan(&self, unit: AngleUnit) -> Result<Number, Error> {
        if self.is_complex() && self.re.is_zero() && (self.im == 1 || self.im == -1) {
            return Err(Error::mp(messages::ATAN_I));
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.atan();
        let out = Number::float(r.real().clone());
        out.apply_angle_unit(unit)
    }

    /// Complex argument (phase) in the active angle unit; negative reals
    /// return +π (doc 03 §4.7).
    pub fn arg(&self, unit: AngleUnit) -> Result<Number, Error> {
        if self.is_zero() {
            return Err(Error::mp(messages::ARG_ZERO));
        }
        if self.is_real() {
            if self.re > 0 {
                let mut n = Number::zero(self.prec());
                n.err = self.err.clone();
                return Ok(n);
            }
            // Negative real: +π in the active unit, exact.
            let mut n = match unit {
                AngleUnit::Radians => Number::pi(self.prec()),
                AngleUnit::Degrees => Number::int(180, self.prec()),
                AngleUnit::Gradians => Number::int(200, self.prec()),
            };
            n.err = self.err.clone();
            return Ok(n);
        }
        let a = self.im.clone().atan2(&self.re);
        let out = Number::float(a);
        out.apply_angle_unit(unit)
    }

    // -- hyperbolic (no angle dependence) --------------------------------

    pub fn sinh(&self) -> Result<Number, Error> {
        self.complex_trig(|z| z.sinh(), self.clone())
    }
    pub fn cosh(&self) -> Result<Number, Error> {
        self.complex_trig(|z| z.cosh(), self.clone())
    }
    pub fn tanh(&self) -> Result<Number, Error> {
        self.complex_trig(|z| z.tanh(), self.clone())
    }

    pub fn asinh(&self) -> Result<Number, Error> {
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.asinh();
        Ok(Number::float(r.real().clone()))
    }

    pub fn acosh(&self) -> Result<Number, Error> {
        if self.is_real() && self.is_finite() && self.re < 1 {
            return Err(Error::mp(messages::ACOSH_DOMAIN));
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.acosh();
        Ok(Number::float(r.real().clone()))
    }

    pub fn atanh(&self) -> Result<Number, Error> {
        if self.is_real() && self.is_finite() && (self.re >= 1 || self.re <= -1) {
            return Err(Error::mp(messages::ATANH_DOMAIN));
        }
        let prec = self.prec();
        let z = Complex::with_val(prec, (&self.re, &self.im));
        let r = z.atanh();
        Ok(Number::float(r.real().clone()))
    }

    // -- internals ---------------------------------------------------------

    fn complex_trig(&self, f: impl Fn(Complex) -> Complex, arg: Number) -> Result<Number, Error> {
        let prec = arg.prec();
        let z = Complex::with_val(prec, (&arg.re, &arg.im));
        let r = f(z);
        let mut out = Number::complex(r.real().clone(), r.imag().clone());
        out.err = self.err.clone();
        check_mpfr_range(&out)
    }

    /// Convert an angle to radians. Complex arguments are passed through in
    /// radians (the identity sin(iy) = i·sinh(y) holds in every unit —
    /// FUNC-T023), matching the upstream tests.
    fn to_radians_angle(&self, unit: AngleUnit) -> Number {
        if self.is_complex() || unit == AngleUnit::Radians {
            return self.clone();
        }
        match unit {
            AngleUnit::Radians => self.clone(),
            AngleUnit::Degrees => {
                let pi = Number::pi(self.prec());
                self.mul(&pi)
                    .div(&Number::int(180, self.prec()))
                    .unwrap_or_else(|_| self.clone())
            }
            AngleUnit::Gradians => {
                let pi = Number::pi(self.prec());
                self.mul(&pi)
                    .div(&Number::int(200, self.prec()))
                    .unwrap_or_else(|_| self.clone())
            }
        }
    }

    /// Convert a radian result back to the active unit.
    fn apply_angle_unit(self, unit: AngleUnit) -> Result<Number, Error> {
        match unit {
            AngleUnit::Radians => Ok(self),
            AngleUnit::Degrees => {
                let pi = Number::pi(self.prec());
                self.mul(&Number::int(180, self.prec())).div(&pi)
            }
            AngleUnit::Gradians => {
                let pi = Number::pi(self.prec());
                self.mul(&Number::int(200, self.prec())).div(&pi)
            }
        }
    }

    /// Exactness guards (doc 03 §4.4). Returns Some(0) when the angle sits on
    /// a documented axis; errors for undefined tangents.
    fn trig_axis_zero(&self, unit: AngleUnit, kind: AxisKind) -> Result<Option<Number>, Error> {
        if !self.is_real() {
            return Ok(None);
        }
        let (full, half) = match unit {
            AngleUnit::Degrees => (180i64, 90i64),
            AngleUnit::Gradians => (200, 100),
            AngleUnit::Radians => return self.trig_axis_zero_radians(kind),
        };
        // Exact rational test when the shadow is available.
        if let Some(r) = self.exact() {
            let full_r = Rational::from(full);
            let q = rat_floor(&(r.clone() / full_r.clone()));
            let m = r.clone() - Rational::from(q) * full_r;
            let is_zero = m.is_zero();
            let is_half = m == half;
            return match kind {
                AxisKind::SinZero => Ok(if is_zero {
                    Some(Number::zero(self.prec()))
                } else {
                    None
                }),
                AxisKind::CosZero => Ok(if is_half {
                    Some(Number::zero(self.prec()))
                } else {
                    None
                }),
                AxisKind::Tan => {
                    if is_half {
                        Err(Error::mp(messages::TANGENT_UNDEF))
                    } else if is_zero {
                        Ok(Some(Number::zero(self.prec())))
                    } else {
                        Ok(None)
                    }
                }
            };
        }
        // Float test on the value itself.
        if !self.re.is_finite() {
            return Ok(None);
        }
        let full_f = Float::with_val(self.prec() + 8, full);
        let q = (self.re.clone() / full_f.clone()).trunc();
        let m = self.re.clone() - q * full_f;
        let m_half = Float::with_val(self.prec() + 8, half);
        let is_zero = m.is_zero();
        let is_half = m == m_half;
        match kind {
            AxisKind::SinZero => Ok(if is_zero {
                Some(Number::zero(self.prec()))
            } else {
                None
            }),
            AxisKind::CosZero => Ok(if is_half {
                Some(Number::zero(self.prec()))
            } else {
                None
            }),
            AxisKind::Tan => {
                if is_half {
                    Err(Error::mp(messages::TANGENT_UNDEF))
                } else if is_zero {
                    Ok(Some(Number::zero(self.prec())))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Radian guards with the documented epsilon |x|·2^(−P) (doc 03 §4.4).
    fn trig_axis_zero_radians(&self, kind: AxisKind) -> Result<Option<Number>, Error> {
        if !self.re.is_finite() {
            return Ok(None);
        }
        let prec = self.prec();
        let w = prec + 8;
        let pi = pi_const(w);
        let x = Float::with_val(w, self.re.clone());
        let xabs = x.clone().abs();
        let two_pow = Float::with_val(w, Integer::from(1) << prec).recip();
        let eps = xabs.clone() * two_pow;

        // Near a multiple of π?
        let k = (x.clone() / pi.clone())
            .round()
            .to_integer()
            .unwrap_or_default();
        let kf = Float::with_val(w, &k);
        let diff = (x.clone() - kf * pi.clone()).abs();
        let near = if xabs.is_zero() {
            diff.is_zero()
        } else {
            diff <= eps
        };

        // Near an odd multiple of π/2?
        let kh = (x.clone() / pi.clone() - Float::with_val(w, 0.5))
            .round()
            .to_integer()
            .unwrap_or_default();
        let khf = Float::with_val(w, &kh);
        let half_pi = pi.clone() / 2;
        let dh: Float = (x.clone() - khf * pi.clone()) - half_pi;
        let dh = dh.abs();
        let is_half = if xabs.is_zero() {
            dh.is_zero()
        } else {
            dh <= eps
        };

        match kind {
            AxisKind::SinZero => Ok(if near { Some(Number::zero(prec)) } else { None }),
            AxisKind::CosZero => Ok(if is_half {
                Some(Number::zero(prec))
            } else {
                None
            }),
            AxisKind::Tan => {
                if is_half {
                    Err(Error::mp(messages::TANGENT_UNDEF))
                } else if near {
                    Ok(Some(Number::zero(prec)))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

enum AxisKind {
    SinZero,
    CosZero,
    Tan,
}

// ----------------------------------------------------------------------
// Factorial and combinatorics (doc 03 §4.5, doc 04 §1.6)
// ----------------------------------------------------------------------

impl Number {
    /// Factorial: Γ(x+1) at precision P (exact for integers while the value
    /// fits P bits); negative integers and poles error; complex unsupported.
    pub fn factorial(&self) -> Result<Number, Error> {
        if self.is_complex() {
            return Err(Error::mp(messages::FACT_COMPLEX));
        }
        if let Some(n) = self.to_integer() {
            if n < 0 {
                return Err(Error::mp(messages::FACT_NEG_INT));
            }
        }
        let prec = self.prec();
        let x1 = self.re.clone() + Float::with_val(prec, 1);
        let g = x1.gamma();
        let mut out = Number::float(g);
        out.err = self.err.clone();
        check_mpfr_range(&out)
    }

    /// `npr(n; r)` = n·(n−1)···(n−r+1), exact integer path (doc 03 §4.5).
    pub fn npr(&self, r: &Number) -> Result<Number, Error> {
        let (n, k) = match (self.to_integer(), r.to_integer()) {
            (Some(n), Some(k)) if self.is_natural() && r.is_natural() => (n, k),
            _ => return Err(Error::mp(messages::PERM_NAT)),
        };
        if k > n {
            return Err(Error::mp(messages::PERM_N_R));
        }
        let nu = n.to_u64().ok_or_else(|| Error::mp(messages::PERM_NAT))?;
        let ku = k.to_u64().ok_or_else(|| Error::mp(messages::PERM_NAT))?;
        let mut acc = Integer::from(1);
        for i in 0..ku {
            acc *= Integer::from(nu - i);
        }
        let mut out = Number::integer(acc, self.prec());
        out.err = self.err.clone().or_else(|| r.err.clone());
        Ok(out)
    }

    /// `ncr(n; r)`: symmetric exact computation (doc 03 §4.5).
    pub fn ncr(&self, r: &Number) -> Result<Number, Error> {
        let (n, k) = match (self.to_integer(), r.to_integer()) {
            (Some(n), Some(k)) if self.is_natural() && r.is_natural() => (n, k),
            _ => return Err(Error::mp(messages::COMB_NAT)),
        };
        if k > n {
            return Err(Error::mp(messages::COMB_N_R));
        }
        let nu = n.to_u64().ok_or_else(|| Error::mp(messages::COMB_NAT))?;
        let ku = k.to_u64().ok_or_else(|| Error::mp(messages::COMB_NAT))?;
        let kk = ku.min(nu - ku);
        let mut acc = Integer::from(1);
        for i in 0..kk {
            acc *= Integer::from(nu - i);
            acc /= Integer::from(i + 1);
        }
        let mut out = Number::integer(acc, self.prec());
        out.err = self.err.clone().or_else(|| r.err.clone());
        Ok(out)
    }

    /// `modexp(x; y; p)`: repeated squaring with `mod` reduction (doc 03 §4.1).
    pub fn modexp(&self, y: &Number, p: &Number) -> Result<Number, Error> {
        let all_int = self.is_integer() && y.is_integer() && p.is_integer() && !p.is_zero();
        if all_int {
            if let (Some(x), Some(e), Some(m)) = (
                self.exact().cloned(),
                y.exact().cloned(),
                p.exact().cloned(),
            ) {
                return modexp_exact(&x, &e, &m, self.prec());
            }
        }
        if self.is_complex() || y.is_complex() || p.is_complex() {
            return Err(Error::mp(messages::MOD_COMPLEX));
        }
        let pow = self.pow_op(y)?;
        pow.mod_op(p)
    }
}

/// Exact modular exponentiation with the documented sign semantics.
/// Rational operands are reduced mod |m| via modular inverses of their
/// denominators (so a negative exponent computes the true modular inverse).
fn modexp_exact(x: &Rational, e: &Rational, m: &Rational, prec: u32) -> Result<Number, Error> {
    let m_abs = abs_rat(m);
    let neg_exp = *e < 0;
    let mut base = rat_mod(&x.clone(), &m_abs)?;
    if neg_exp {
        if base.numer().to_u64() == Some(0) {
            return Err(Error::mp(messages::DIV_BY_ZERO));
        }
        base = rat_mod(&base.clone().recip(), &m_abs)?;
    }
    let exp_int = crate::number::rat_trunc(&abs_rat(e));
    let mut result = Rational::from(1);
    let mut v = exp_int;
    while v > 0 {
        if (v.clone() & 1) == 1 {
            result = rat_mod(&(result * base.clone()), &m_abs)?;
        }
        base = rat_mod(&(base.clone() * base.clone()), &m_abs)?;
        v >>= 1;
    }
    // Negative modulus behaves as `mod` with the divisor's sign.
    if *m < 0 && !result.is_zero() {
        result -= m_abs.clone();
    }
    Ok(Number::rational(result, prec))
}

/// Rational mod |m| via the modular inverse of the denominator; errors when
/// the denominator is not invertible mod |m|.
fn rat_mod(a: &Rational, m: &Rational) -> Result<Rational, Error> {
    if a.is_integer() {
        return Ok(int_mod_floor_r(a, m));
    }
    // a = p/q reduced: a mod M = (p mod M)·q⁻¹ mod M.
    let m_int = rat_trunc_n(m);
    let p = int_mod_floor(a.numer(), &m_int);
    let q = a.denom().clone();
    let q_inv = mod_inverse(&q, &m_int).ok_or_else(|| Error::mp(messages::DIV_BY_ZERO))?;
    Ok(int_mod_floor_r(&Rational::from(p * q_inv), m))
}

fn rat_trunc_n(r: &Rational) -> Integer {
    r.numer().clone() / r.denom().clone()
}

/// Modular inverse of a mod m (m > 0, gcd(a, m) = 1) via extended Euclid.
fn mod_inverse(a: &Integer, m: &Integer) -> Option<Integer> {
    let a = int_mod_floor(a, m);
    if a.is_zero() {
        return None;
    }
    let (mut old_r, mut r) = (a.clone(), m.clone());
    let (mut old_s, mut s) = (Integer::from(1), Integer::from(0));
    while !r.is_zero() {
        let q = old_r.clone() / r.clone();
        let nr = old_r.clone() - q.clone() * r.clone();
        old_r = r;
        r = nr;
        let ns = old_s.clone() - q * s.clone();
        old_s = s;
        s = ns;
    }
    if old_r.clone().abs().to_u64() != Some(1) {
        return None; // not coprime
    }
    let inv = if old_s < 0 { old_s + m.clone() } else { old_s };
    Some(inv)
}

fn int_mod_floor_r(a: &Rational, m: &Rational) -> Rational {
    let q = rat_floor(&(a.clone() / m.clone()));
    let r = a.clone() - Rational::from(q) * m.clone();
    if !r.is_zero() && ((r < 0) != (*m < 0)) {
        r + m.clone()
    } else {
        r
    }
}

// ----------------------------------------------------------------------
// Rounding family (doc 03 §4.6) and statistics (doc 04 §1.5)
// ----------------------------------------------------------------------

impl Number {
    /// `[x]` / `round`: nearest integer, ties away from zero.
    pub fn round(&self) -> Result<Number, Error> {
        if self.is_complex() {
            return Err(Error::mp(messages::ROUND_COMPLEX));
        }
        if let Some(r) = self.exact() {
            let (num, den) = (r.numer(), r.denom());
            let two_p = num.clone() * Integer::from(2);
            let two_p = if two_p < 0 { -two_p } else { two_p };
            let n = (two_p + den.clone()) / (den.clone() * Integer::from(2));
            let n = if *r < 0 { -n } else { n };
            let mut out = Number::zero(self.prec());
            out.err = self.err.clone();
            return Ok(out.finish_exact(Rational::from(n)));
        }
        let mut out = Number::float(self.re.clone().round());
        out.err = self.err.clone();
        Ok(out)
    }

    /// ⌊x⌋ floor toward −∞.
    pub fn floor(&self) -> Result<Number, Error> {
        if self.is_complex() {
            return Err(Error::mp(messages::FLOOR_COMPLEX));
        }
        if let Some(r) = self.exact() {
            let mut out = Number::zero(self.prec());
            out.err = self.err.clone();
            return Ok(out.finish_exact(Rational::from(rat_floor(r))));
        }
        let mut out = Number::float(self.re.clone().floor());
        out.err = self.err.clone();
        Ok(out)
    }

    /// ⌈x⌉ ceiling toward +∞.
    pub fn ceil(&self) -> Result<Number, Error> {
        if self.is_complex() {
            return Err(Error::mp(messages::CEIL_COMPLEX));
        }
        if let Some(r) = self.exact() {
            let c = crate::number::rat_ceil(r);
            let mut out = Number::zero(self.prec());
            out.err = self.err.clone();
            return Ok(out.finish_exact(Rational::from(c)));
        }
        let mut out = Number::float(self.re.clone().ceil());
        out.err = self.err.clone();
        Ok(out)
    }

    /// `int`: integer component, truncate toward 0.
    pub fn int_component(&self) -> Result<Number, Error> {
        if let Some(r) = self.exact() {
            let mut out = Number::zero(self.prec());
            out.err = self.err.clone();
            return Ok(out.finish_exact(Rational::from(r.numer().clone() / r.denom().clone())));
        }
        let mut out = Number::float(self.re.clone().trunc());
        out.err = self.err.clone();
        Ok(out)
    }

    /// `frac`: fractional component x − trunc(x) — sign follows x; differs
    /// from `{ }` (doc 03 §4.6).
    pub fn frac_component(&self) -> Result<Number, Error> {
        if let Some(r) = self.exact() {
            let t = r.numer().clone() / r.denom().clone();
            let f = r.clone() - Rational::from(t);
            let mut out = Number::zero(self.prec());
            out.err = self.err.clone();
            return Ok(out.finish_exact(f));
        }
        let mut out = Number::float(self.re.clone() - self.re.clone().trunc());
        out.err = self.err.clone();
        Ok(out)
    }

    /// `{x}`: fractional part x − floor(x), always in [0,1) (doc 03 §4.6).
    pub fn fractional_part(&self) -> Result<Number, Error> {
        if let Some(r) = self.exact() {
            let f = r.clone() - Rational::from(rat_floor(r));
            let mut out = Number::zero(self.prec());
            out.err = self.err.clone();
            return Ok(out.finish_exact(f));
        }
        let mut out = Number::float(self.re.clone() - self.re.clone().floor());
        out.err = self.err.clone();
        Ok(out)
    }

    /// Square root (complex for negatives, doc 03 §4.2).
    pub fn sqrt(&self) -> Result<Number, Error> {
        self.root(2)
    }

    // -- statistics --------------------------------------------------------

    /// Σxᵢ
    pub fn sum(args: &[Number]) -> Result<Number, Error> {
        let mut acc = args
            .first()
            .cloned()
            .ok_or_else(|| Error::new(ErrorCode::Mp))?;
        for a in &args[1..] {
            acc = acc.add(a);
        }
        Ok(acc)
    }

    /// Σxᵢ²
    pub fn sumsq(args: &[Number]) -> Result<Number, Error> {
        let first = args.first().ok_or_else(|| Error::new(ErrorCode::Mp))?;
        let mut acc = first.mul(first);
        for a in &args[1..] {
            acc = acc.add(&a.mul(a));
        }
        Ok(acc)
    }

    /// Σxᵢ/n
    pub fn average(args: &[Number]) -> Result<Number, Error> {
        let n = Number::int(args.len() as i64, args[0].prec());
        Number::sum(args)?.div(&n)
    }

    /// Middle value (sorted); mean of the two middle values for even n.
    pub fn median(args: &[Number]) -> Result<Number, Error> {
        if args.iter().any(|a| a.is_complex()) {
            return Err(Error::mp(messages::MEDIAN_COMPLEX));
        }
        let mut v: Vec<&Number> = args.iter().collect();
        v.sort_by(|a, b| a.cmp_value(b));
        let n = v.len();
        if n % 2 == 1 {
            Ok(v[n / 2].clone())
        } else {
            let m = v[n / 2 - 1].add(v[n / 2]);
            m.div(&Number::int(2, args[0].prec()))
        }
    }

    pub fn min(args: &[Number]) -> Result<Number, Error> {
        if args.iter().any(|a| a.is_complex()) {
            return Err(Error::mp(messages::MIN_COMPLEX));
        }
        let mut best = &args[0];
        for a in &args[1..] {
            if a.cmp_value(best) == std::cmp::Ordering::Less {
                best = a;
            }
        }
        Ok(best.clone())
    }

    pub fn max(args: &[Number]) -> Result<Number, Error> {
        if args.iter().any(|a| a.is_complex()) {
            return Err(Error::mp(messages::MAX_COMPLEX));
        }
        let mut best = &args[0];
        for a in &args[1..] {
            if a.cmp_value(best) == std::cmp::Ordering::Greater {
                best = a;
            }
        }
        Ok(best.clone())
    }

    /// Sample variance Σ|xᵢ−mean|²/(n−1); requires ≥ 2 arguments.
    pub fn var(args: &[Number]) -> Result<Number, Error> {
        if args.len() < 2 {
            return Err(Error::mp(crate::error::messages::takes_at_least("var", 2)));
        }
        variance(args, false)
    }

    /// Population variance Σ|xᵢ−mean|²/n.
    pub fn varp(args: &[Number]) -> Result<Number, Error> {
        variance(args, true)
    }

    /// √(sample variance); requires ≥ 2 arguments.
    pub fn stdev(args: &[Number]) -> Result<Number, Error> {
        Number::var(args)?.sqrt()
    }

    /// √(population variance).
    pub fn stdevp(args: &[Number]) -> Result<Number, Error> {
        Number::varp(args)?.sqrt()
    }
}

fn variance(args: &[Number], population: bool) -> Result<Number, Error> {
    let mean = Number::average(args)?;
    let mut acc = Number::zero(args[0].prec());
    for a in args {
        let d = a.sub(&mean);
        // |x−mean|² = d·conj(d) — real d² for reals, modulus² for complex.
        let d2 = d.mul(&d.conj());
        acc = acc.add(&d2);
    }
    let n = if population {
        args.len()
    } else {
        args.len() - 1
    };
    acc.div(&Number::int(n as i64, args[0].prec()))
}

// ----------------------------------------------------------------------
// gcd / lcm (doc 04 §1.6)
// ----------------------------------------------------------------------

/// gcd of 1..n integer arguments (1 arg = |x|); 0 and non-integers error.
pub fn gcd(args: &[Number]) -> Result<Number, Error> {
    if args.is_empty() {
        return Err(Error::mp(crate::error::messages::takes_at_least("gcd", 1)));
    }
    let mut ints = Vec::new();
    for a in args {
        match a.to_integer() {
            Some(i) if !i.is_zero() => ints.push(i),
            _ => return Err(Error::mp(crate::error::messages::GCD_INT)),
        }
    }
    let mut g = ints[0].clone();
    if g < 0 {
        g = -g;
    }
    for v in &ints[1..] {
        g = g.gcd(v);
    }
    let mut out = Number::integer(g, args[0].prec());
    out.err = args.iter().find_map(|a| a.err.clone());
    Ok(out)
}

/// lcm of 1..n integer arguments.
pub fn lcm(args: &[Number]) -> Result<Number, Error> {
    if args.is_empty() {
        return Err(Error::mp(crate::error::messages::takes_at_least("lcm", 1)));
    }
    let mut ints = Vec::new();
    for a in args {
        match a.to_integer() {
            Some(i) if !i.is_zero() => ints.push(i),
            _ => return Err(Error::mp(crate::error::messages::LCM_INT)),
        }
    }
    let mut l = ints[0].clone();
    if l < 0 {
        l = -l;
    }
    for v in &ints[1..] {
        l = l.lcm(v);
    }
    let mut out = Number::integer(l, args[0].prec());
    out.err = args.iter().find_map(|a| a.err.clone());
    Ok(out)
}

// ----------------------------------------------------------------------
// Range checks
// ----------------------------------------------------------------------

/// Map MPFR overflow on the result to the documented error (doc 03 §5).
pub(crate) fn check_mpfr_range(n: &Number) -> Result<Number, Error> {
    if !n.re.is_finite() || !n.im.is_finite() {
        return Err(Error::mp(messages::OVERFLOW));
    }
    Ok(n.clone())
}
