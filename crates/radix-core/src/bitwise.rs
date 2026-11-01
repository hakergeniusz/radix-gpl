//! Bitwise operations, shifts, word-size semantics, factorization
//! (spec: doc 02 §7, doc 03 §4.7, §5).

use crate::error::{messages, Error};
use crate::number::{int_mod_floor, Number};
use rug::Integer;

/// Operand guard: bitwise ops need non-negative integers (doc 02 §7).
fn nat_operand(x: &Number, op: &str) -> Result<Integer, Error> {
    if let Some(i) = x.to_integer() {
        if i >= 0 {
            return Ok(i);
        }
    }
    Err(Error::mp(format!(
        "Bitwise {op} is only defined for non-negative integers"
    )))
}

/// `and`/`∧`: nibble-wise AND, result width = max (unmasked, doc 02 §7).
pub fn and(a: &Number, b: &Number) -> Result<Number, Error> {
    combine(a, b, "AND", |x, y| x & y)
}

/// `or`/`∨`.
pub fn or(a: &Number, b: &Number) -> Result<Number, Error> {
    combine(a, b, "OR", |x, y| x | y)
}

/// `xor`/`⊻`/`⊕`.
pub fn xor(a: &Number, b: &Number) -> Result<Number, Error> {
    combine(a, b, "XOR", |x, y| x ^ y)
}

fn combine(
    a: &Number,
    b: &Number,
    op: &str,
    f: impl Fn(Integer, &Integer) -> Integer,
) -> Result<Number, Error> {
    let x = nat_operand(a, op)?;
    let y = nat_operand(b, op)?;
    let mut out = Number::integer(f(x, &y), a.prec().max(b.prec()));
    out.err = a.err.clone().or_else(|| b.err.clone());
    Ok(out)
}

/// Mask of the word size: 2^wordlen − 1.
fn word_mask(wordlen: u32) -> Integer {
    (Integer::from(1) << wordlen) - 1
}

/// Word-size masked ops: NOT/NAND/NOR/XNOR (result width = word size,
/// doc 02 §7).
pub fn not(a: &Number, wordlen: u32) -> Result<Number, Error> {
    let x = nat_operand(a, "NOT")?;
    let v = !x & word_mask(wordlen);
    let mut out = Number::integer(v, a.prec());
    out.err = a.err.clone();
    Ok(out)
}

pub fn nand(a: &Number, b: &Number, wordlen: u32) -> Result<Number, Error> {
    let x = nat_operand(a, "NAND")?;
    let y = nat_operand(b, "NAND")?;
    let v = !(x & &y) & word_mask(wordlen);
    let mut out = Number::integer(v, a.prec().max(b.prec()));
    out.err = a.err.clone().or_else(|| b.err.clone());
    Ok(out)
}

pub fn nor(a: &Number, b: &Number, wordlen: u32) -> Result<Number, Error> {
    let x = nat_operand(a, "NOR")?;
    let y = nat_operand(b, "NOR")?;
    let v = !(x | &y) & word_mask(wordlen);
    let mut out = Number::integer(v, a.prec().max(b.prec()));
    out.err = a.err.clone().or_else(|| b.err.clone());
    Ok(out)
}

pub fn xnor(a: &Number, b: &Number, wordlen: u32) -> Result<Number, Error> {
    let x = nat_operand(a, "XNOR")?;
    let y = nat_operand(b, "XNOR")?;
    let v = !(x ^ &y) & word_mask(wordlen);
    let mut out = Number::integer(v, a.prec().max(b.prec()));
    out.err = a.err.clone().or_else(|| b.err.clone());
    Ok(out)
}

/// Operand fits the word size: unsigned range for non-negative, signed range
/// for negatives (doc 02 §7.6).
fn fits_word(x: &Integer, wordlen: u32) -> bool {
    if *x >= 0 {
        *x < (Integer::from(1) << wordlen)
    } else {
        *x >= -(Integer::from(1) << (wordlen - 1))
    }
}

/// Shift kind (doc 02 §7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftKind {
    Left,
    Right,
    UnsignedRight,
}

/// Shifts (doc 02 §7.6): count reduced mod word size; left shift of a
/// negative sign-extends; right shift is arithmetic (sign-preserving) for
/// negatives, logical for non-negative; unsigned right masks to the word
/// size first and produces a non-negative result.
pub fn shift(a: &Number, count: &Number, wordlen: u32, kind: ShiftKind) -> Result<Number, Error> {
    if !a.is_integer() || !count.is_integer() {
        return Err(Error::mp(messages::SHIFT_INT));
    }
    let x = a
        .to_integer()
        .ok_or_else(|| Error::mp(messages::SHIFT_INT))?;
    let c = count
        .to_integer()
        .ok_or_else(|| Error::mp(messages::SHIFT_INT))?;
    if !fits_word(&x, wordlen) || !fits_word(&c, wordlen) {
        return Err(Error::mp(messages::OVERFLOW_WORD));
    }
    // Shift count reduced modulo the word size.
    let cu = int_mod_floor(&c, &Integer::from(wordlen as u64))
        .to_usize()
        .unwrap_or(0);
    let out_val = match kind {
        ShiftKind::Left => {
            if x < 0 {
                -((-x) << cu)
            } else {
                x << cu
            }
        }
        ShiftKind::Right => x >> cu,
        ShiftKind::UnsignedRight => {
            let masked = x & word_mask(wordlen);
            masked >> cu
        }
    };
    // The shifted result must fit the word size (NUM-T028: 255«1 at 8 bits
    // overflows instead of silently widening).
    if !fits_word(&out_val, wordlen) {
        return Err(Error::mp(messages::OVERFLOW_WORD));
    }
    let mut out = Number::integer(out_val, a.prec().max(count.prec()));
    out.err = a.err.clone().or_else(|| count.err.clone());
    Ok(out)
}

/// `twos(x)`: two's complement within the word size = `not(x) + 1`
/// (doc 04 §1.3).
pub fn twos(a: &Number, wordlen: u32) -> Result<Number, Error> {
    let x = match a.to_integer() {
        Some(i) if i >= 0 => i,
        _ => return Err(Error::mp(messages::TWOS_NAT)),
    };
    let mask = word_mask(wordlen);
    let v = ((x ^ &mask) + 1) & mask;
    let mut out = Number::integer(v, a.prec());
    out.err = a.err.clone();
    Ok(out)
}

/// `bswap(x)`: byte-swap within the word size, sign-extending negatives
/// (doc 04 §1.3).
pub fn bswap(a: &Number, wordlen: u32) -> Result<Number, Error> {
    let x = match a.to_integer() {
        Some(i) => i,
        None => return Err(Error::mp(messages::BSWAP_INT)),
    };
    if !fits_word(&x, wordlen) {
        return Err(Error::mp(messages::OVERFLOW_WORD));
    }
    let nbytes = (wordlen / 8) as usize;
    // Two's-complement bytes within the word (x & mask is x mod 2^wordlen).
    let v = x & word_mask(wordlen);
    let mut swapped = Integer::from(0);
    for i in 0..nbytes {
        let byte = (v.clone() >> (8 * i)) & 0xFF;
        swapped |= byte << (8 * (nbytes - 1 - i));
    }
    // Sign-extend: if the top bit of the swapped value is set, subtract 2^w.
    let top = swapped.clone() >> (wordlen - 1);
    if top == 1 {
        swapped -= Integer::from(1) << wordlen;
    }
    let mut out = Number::integer(swapped, a.prec());
    out.err = a.err.clone();
    Ok(out)
}

/// Reinterpret a value in two's complement of a new word size (doc 03 §5).
/// Floats and complex values are left alone.
pub fn reinterpret_word_size(x: &Number, new_wordlen: u32) -> Number {
    if x.is_complex() || !x.is_integer() {
        return x.clone();
    }
    let v = match x.to_integer() {
        Some(v) => v,
        None => return x.clone(),
    };
    let m = Integer::from(1) << new_wordlen;
    let mut v = int_mod_floor(&v, &m);
    if v >= (Integer::from(1) << (new_wordlen - 1)) {
        v -= m;
    }
    let mut out = Number::integer(v, x.prec());
    out.err = x.err.clone();
    out
}

// ----------------------------------------------------------------------
// Factorization (doc 03 §4.7)
// ----------------------------------------------------------------------

/// Prime factorization with multiplicity, ascending; the sign is applied to
/// the first factor for negative input; 0 → [0]; 1 → [1].
pub fn factorize(x: &Number) -> Result<Vec<Integer>, Error> {
    let v = match x.to_integer() {
        Some(v) => v,
        None => return Err(Error::mp(messages::NEED_INTEGER_FACTORIZATION)),
    };
    if v.to_u64() == Some(0) {
        return Ok(vec![Integer::from(0)]);
    }
    let neg = v < 0;
    let mut n = if neg { -v } else { v };
    let mut factors = Vec::new();
    if n == 1 {
        factors.push(if neg {
            Integer::from(-1)
        } else {
            Integer::from(1)
        });
        return Ok(factors);
    }
    for p in SMALL_PRIMES.iter() {
        let p = Integer::from(*p);
        while (n.clone() % p.clone()).is_zero() {
            factors.push(p.clone());
            n /= p.clone();
        }
        if n == 1 {
            break;
        }
    }
    if n > 1 {
        pollard_rho_factor(&n, &mut factors);
    }
    if neg && !factors.is_empty() {
        factors[0] = -factors[0].clone();
    }
    Ok(factors)
}

const SMALL_PRIMES: &[u64] = &[
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193,
    197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307,
    311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 379, 383, 389, 397, 401, 409, 419, 421,
    431, 433, 439, 443, 449, 457, 461, 463, 467, 479, 487, 491, 499, 503, 509, 521, 523, 541,
];

fn is_probable_prime(n: &Integer) -> bool {
    n.is_probably_prime(25) != rug::integer::IsPrime::No
}

/// Pollard's rho with recursion on composite cofactors.
fn pollard_rho_factor(n: &Integer, out: &mut Vec<Integer>) {
    if n.to_u64() == Some(1) {
        return;
    }
    if is_probable_prime(n) {
        out.push(n.clone());
        return;
    }
    let d = pollard_rho_find(n);
    if d.is_zero() {
        // Fallback: trial division upward (rare path for small n).
        let mut f = Integer::from(SMALL_PRIMES.last().unwrap() + 2);
        loop {
            if (n.clone() % f.clone()).is_zero() {
                out.push(f.clone());
                let rest = n / f;
                pollard_rho_factor(&rest, out);
                return;
            }
            f += 2;
        }
        #[allow(unreachable_code)]
        {
            return;
        }
    }
    pollard_rho_factor(&d, out);
    pollard_rho_factor(&(n.clone() / d.clone()), out);
}

fn pollard_rho_find(n: &Integer) -> Integer {
    if (n.clone() % Integer::from(2)).is_zero() {
        return Integer::from(2);
    }
    // Deterministic seeds; retry with different c on failure.
    for c in 1u64..50 {
        let mut x = Integer::from(2);
        let mut y = Integer::from(2);
        let cc = Integer::from(c);
        let mut d;
        loop {
            x = (x.clone() * x.clone() + cc.clone()) % n;
            y = (y.clone() * y.clone() + cc.clone()) % n;
            y = (y.clone() * y.clone() + cc.clone()) % n;
            let diff = if x > y {
                x.clone() - y.clone()
            } else {
                y.clone() - x.clone()
            };
            d = diff.gcd(n);
            if d != 1 {
                break;
            }
        }
        if d != *n {
            return d;
        }
    }
    Integer::from(0)
}
