//! Financial calculations (spec: doc 04 §4).
//!
//! Dialog-only computations; results replace the display and are not added
//! to history. Formulas exactly as documented; domain errors surface as
//! standard MP errors.

use crate::error::{messages, Error};
use crate::number::Number;

fn ln_checked(v: &Number) -> Result<Number, Error> {
    if !v.is_real() || v.re <= 0 {
        return Err(Error::mp(messages::LOG_ZERO));
    }
    v.ln()
}

/// Dialog-only financial dispatch (doc 04 §4: "financial functions cannot
/// be performed using the keyboard" — the names are deliberately not in the
/// function catalog; the GUI dialog calls this directly).
pub fn evaluate(name: &str, args: &[Number]) -> Result<Number, Error> {
    let wrong_arity = || {
        Error::with_message(
            crate::error::ErrorCode::Invalid,
            format!("{name}: wrong number of arguments"),
        )
    };
    let a = |i: usize| -> Result<&Number, Error> { args.get(i).ok_or_else(wrong_arity) };
    match name {
        "Ctrm" => ctrm(a(0)?, a(1)?, a(2)?),
        "Ddb" => {
            let (v, status) = ddb(a(0)?, a(1)?, a(2)?)?;
            if let Some(e) = status {
                return Ok(v.with_error(e));
            }
            Ok(v)
        }
        "Fv" => fv(a(0)?, a(1)?, a(2)?),
        "Gpm" => gpm(a(0)?, a(1)?),
        "Pmt" => pmt(a(0)?, a(1)?, a(2)?),
        "Pv" => pv(a(0)?, a(1)?, a(2)?),
        "Rate" => rate(a(0)?, a(1)?, a(2)?),
        "Sln" => sln(a(0)?, a(1)?, a(2)?),
        "Syd" => syd(a(0)?, a(1)?, a(2)?, a(3)?),
        "Term" => term(a(0)?, a(1)?, a(2)?),
        _ => Err(Error::with_message(
            crate::error::ErrorCode::UnknownFunction,
            messages::unknown_function(name),
        )),
    }
}

/// Ctrm: ln(fv/pv) / ln(1+pint).
pub fn ctrm(pint: &Number, fv: &Number, pv: &Number) -> Result<Number, Error> {
    let ratio = fv.div(pv)?;
    let num = ln_checked(&ratio)?;
    let den = ln_checked(&pint.add(&Number::one(pint.prec())))?;
    num.div(&den)
}

/// Ddb: iterate `VAL = (cost − bv)·2/life`, `bv += VAL` for `period`
/// periods (period truncated; negative period → status message, 0 iterations).
pub fn ddb(
    cost: &Number,
    life: &Number,
    period: &Number,
) -> Result<(Number, Option<Error>), Error> {
    let mut status = None;
    let periods = match period.to_integer() {
        Some(p) if p >= 0 => p,
        Some(_) => {
            status = Some(Error::mp("Error: the number of periods must be positive"));
            rug::Integer::from(0)
        }
        None => period.to_integer().unwrap_or_default(),
    };
    let two = Number::int(2, cost.prec());
    let mut bv = Number::zero(cost.prec());
    // VAL of the last iterated period (0 iterations → 0).
    let mut val = Number::zero(cost.prec());
    let mut n = periods;
    while n > 0 {
        val = cost.sub(&bv).mul(&two).div(life)?;
        bv = bv.add(&val);
        n -= 1;
    }
    Ok((val, status))
}

/// Fv: pmt·((1+pint)ⁿ − 1)/pint.
pub fn fv(pmt: &Number, pint: &Number, n: &Number) -> Result<Number, Error> {
    if pint.is_zero() {
        return Err(Error::mp(messages::DIV_BY_ZERO));
    }
    let one = Number::one(pint.prec());
    let grown = one.add(pint).pow_op(n)?;
    pmt.mul(&grown.sub(&Number::one(pint.prec()))).div(pint)
}

/// Gpm: cost/(1−margin).
pub fn gpm(cost: &Number, margin: &Number) -> Result<Number, Error> {
    let one = Number::one(cost.prec());
    cost.div(&one.sub(margin))
}

/// Pmt: prin·pint/(1−(1+pint)^(−n)).
pub fn pmt(prin: &Number, pint: &Number, n: &Number) -> Result<Number, Error> {
    if pint.is_zero() {
        return Err(Error::mp(messages::DIV_BY_ZERO));
    }
    let one = Number::one(pint.prec());
    let neg_n = n.neg();
    let grown = one.add(pint).pow_op(&neg_n)?;
    let denom = one.sub(&grown);
    prin.mul(pint).div(&denom)
}

/// Pv: pmt·(1−(1+pint)^(−n))/pint.
pub fn pv(pmt: &Number, pint: &Number, n: &Number) -> Result<Number, Error> {
    if pint.is_zero() {
        return Err(Error::mp(messages::DIV_BY_ZERO));
    }
    let one = Number::one(pint.prec());
    let neg_n = n.neg();
    let grown = one.add(pint).pow_op(&neg_n)?;
    let num = one.sub(&grown);
    pmt.mul(&num).div(pint)
}

/// Rate: (fv/pv)^(1/n) − 1.
pub fn rate(fv: &Number, pv: &Number, n: &Number) -> Result<Number, Error> {
    let ratio = fv.div(pv)?;
    let one = Number::one(fv.prec());
    let root = ratio.pow_op(&one.div(n)?)?;
    Ok(root.sub(&one))
}

/// Sln: (cost−salvage)/life.
pub fn sln(cost: &Number, salvage: &Number, life: &Number) -> Result<Number, Error> {
    cost.sub(salvage).div(life)
}

/// Syd: (cost−salvage)·(life−period+1)/((life·(life+1))/2).
pub fn syd(
    cost: &Number,
    salvage: &Number,
    life: &Number,
    period: &Number,
) -> Result<Number, Error> {
    let one = Number::one(cost.prec());
    let two = Number::int(2, cost.prec());
    let numerator = cost.sub(salvage).mul(&life.sub(period).add(&one));
    let denominator = life.mul(&life.add(&one)).div(&two)?;
    numerator.div(&denominator)
}

/// Term: ln(1 + fv·pint/pmt)/ln(1+pint).
pub fn term(pmt: &Number, fv: &Number, pint: &Number) -> Result<Number, Error> {
    let inner = fv.mul(pint).div(pmt)?;
    let one = Number::one(pmt.prec());
    let num = ln_checked(&inner.add(&one))?;
    let den = ln_checked(&pint.add(&one))?;
    num.div(&den)
}
