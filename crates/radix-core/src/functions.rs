//! Built-in function registry (spec: doc 04 §1).

use crate::error::{messages, Error};
use crate::number::Number;
use crate::numfuncs::{self, AngleUnit};

/// Function category for the UI popover (doc 04 §1.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncCategory {
    Trigonometry,
    Complex,
    Programming,
    Rounding,
    Statistics,
    Sundry,
}

/// Argument-count rule.
#[derive(Debug, Clone, Copy)]
pub enum Arity {
    Exact(u32),
    /// 1 or 2 (`log`).
    OneOrTwo,
    /// At least N (variadic functions).
    AtLeast(u32),
}

/// How the function evaluates.
#[derive(Clone)]
pub enum FuncImpl {
    /// Angle-unit aware (trig + arg).
    Trig(TrigFn),
    Unary(fn(&Number, &Env) -> Result<Number, Error>),
    Variadic(fn(&[Number], &Env) -> Result<Number, Error>),
}

#[derive(Clone, Copy)]
pub enum TrigFn {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Arg,
}

#[derive(Clone)]
pub struct FuncDef {
    pub name: &'static str,
    pub category: FuncCategory,
    pub arity: Arity,
    pub inverse_of: Option<&'static str>,
    pub description: &'static str,
    pub op: FuncImpl,
}

macro_rules! plain {
    ($f:expr) => {
        FuncImpl::Unary(|x, _env| {
            let f: fn(&Number) -> Result<Number, Error> = $f;
            f(x)
        })
    };
}

/// User-function call back into the evaluator (recursion, VAR-T011).
pub type CallFn<'e> = dyn Fn(&str, &[Number]) -> Result<Number, Error> + 'e;

/// The environment a function sees (a view over the evaluator state).
pub struct Env<'e> {
    pub wordlen: u32,
    pub angle_unit: AngleUnit,
    pub precision: u32,
    pub call: &'e CallFn<'e>,
}

/// The built-in catalog. Aliases (asin-style) are registered as separate
/// entries mapping to the same operations (doc 04 §1.1 note).
pub fn builtin_functions() -> Vec<FuncDef> {
    use FuncCategory as C;

    vec![
        // -- trigonometry -------------------------------------------------
        FuncDef {
            name: "sin",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "sin x",
            op: FuncImpl::Trig(TrigFn::Sin),
        },
        FuncDef {
            name: "cos",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "cos x",
            op: FuncImpl::Trig(TrigFn::Cos),
        },
        FuncDef {
            name: "tan",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "tan x",
            op: FuncImpl::Trig(TrigFn::Tan),
        },
        FuncDef {
            name: "sin⁻¹",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("sin"),
            description: "sin⁻¹ x",
            op: FuncImpl::Trig(TrigFn::Asin),
        },
        FuncDef {
            name: "asin",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("sin"),
            description: "sin⁻¹ x",
            op: FuncImpl::Trig(TrigFn::Asin),
        },
        FuncDef {
            name: "cos⁻¹",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("cos"),
            description: "cos⁻¹ x",
            op: FuncImpl::Trig(TrigFn::Acos),
        },
        FuncDef {
            name: "acos",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("cos"),
            description: "cos⁻¹ x",
            op: FuncImpl::Trig(TrigFn::Acos),
        },
        FuncDef {
            name: "tan⁻¹",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("tan"),
            description: "tan⁻¹ x",
            op: FuncImpl::Trig(TrigFn::Atan),
        },
        FuncDef {
            name: "atan",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("tan"),
            description: "tan⁻¹ x",
            op: FuncImpl::Trig(TrigFn::Atan),
        },
        FuncDef {
            name: "sinh",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "sinh x",
            op: plain!(|x| x.sinh()),
        },
        FuncDef {
            name: "cosh",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "cosh x",
            op: plain!(|x| x.cosh()),
        },
        FuncDef {
            name: "tanh",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "tanh x",
            op: plain!(|x| x.tanh()),
        },
        FuncDef {
            name: "sinh⁻¹",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("sinh"),
            description: "sinh⁻¹ x",
            op: plain!(|x| x.asinh()),
        },
        FuncDef {
            name: "asinh",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("sinh"),
            description: "sinh⁻¹ x",
            op: plain!(|x| x.asinh()),
        },
        FuncDef {
            name: "cosh⁻¹",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("cosh"),
            description: "cosh⁻¹ x",
            op: plain!(|x| x.acosh()),
        },
        FuncDef {
            name: "acosh",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("cosh"),
            description: "cosh⁻¹ x",
            op: plain!(|x| x.acosh()),
        },
        FuncDef {
            name: "tanh⁻¹",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("tanh"),
            description: "tanh⁻¹ x",
            op: plain!(|x| x.atanh()),
        },
        FuncDef {
            name: "atanh",
            category: C::Trigonometry,
            arity: Arity::Exact(1),
            inverse_of: Some("tanh"),
            description: "tanh⁻¹ x",
            op: plain!(|x| x.atanh()),
        },
        // -- complex --------------------------------------------------------
        FuncDef {
            name: "conj",
            category: C::Complex,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "conjugate",
            op: plain!(|x| Ok(x.conj())),
        },
        FuncDef {
            name: "arg",
            category: C::Complex,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "complex argument",
            op: FuncImpl::Trig(TrigFn::Arg),
        },
        FuncDef {
            name: "re",
            category: C::Complex,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "real component",
            op: plain!(|x| Ok(x.re_part())),
        },
        FuncDef {
            name: "im",
            category: C::Complex,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "imaginary component",
            op: plain!(|x| Ok(x.im_part())),
        },
        // -- programming ------------------------------------------------------
        FuncDef {
            name: "twos",
            category: C::Programming,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "two’s complement",
            op: FuncImpl::Unary(|x, env| crate::bitwise::twos(x, env.wordlen)),
        },
        FuncDef {
            name: "bswap",
            category: C::Programming,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "swap endianness",
            op: FuncImpl::Unary(|x, env| crate::bitwise::bswap(x, env.wordlen)),
        },
        FuncDef {
            name: "modexp",
            category: C::Programming,
            arity: Arity::Exact(3),
            inverse_of: None,
            description: "modular exponentiation",
            op: FuncImpl::Variadic(|args, _env| args[0].modexp(&args[1], &args[2])),
        },
        FuncDef {
            name: "mod",
            category: C::Programming,
            arity: Arity::Exact(2),
            inverse_of: None,
            description: "modulus divide",
            op: FuncImpl::Variadic(|args, _env| args[0].mod_op(&args[1])),
        },
        // -- rounding ---------------------------------------------------------
        FuncDef {
            name: "round",
            category: C::Rounding,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "round",
            op: plain!(|x| x.round()),
        },
        FuncDef {
            name: "floor",
            category: C::Rounding,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "floor",
            op: plain!(|x| x.floor()),
        },
        FuncDef {
            name: "ceil",
            category: C::Rounding,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "ceiling",
            op: plain!(|x| x.ceil()),
        },
        FuncDef {
            name: "int",
            category: C::Rounding,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "integer component",
            op: plain!(|x| x.int_component()),
        },
        FuncDef {
            name: "frac",
            category: C::Rounding,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "fractional component",
            op: plain!(|x| x.frac_component()),
        },
        // -- sundry -----------------------------------------------------------
        FuncDef {
            name: "ln",
            category: C::Sundry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "natural logarithm",
            op: plain!(|x| x.ln()),
        },
        FuncDef {
            name: "sqrt",
            category: C::Sundry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "square root",
            op: plain!(|x| x.sqrt()),
        },
        FuncDef {
            name: "abs",
            category: C::Sundry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "absolute value",
            op: plain!(|x| Ok(x.abs())),
        },
        FuncDef {
            name: "sgn",
            category: C::Sundry,
            arity: Arity::Exact(1),
            inverse_of: None,
            description: "signum",
            op: plain!(|x| x.sgn()),
        },
        FuncDef {
            name: "log",
            category: C::Sundry,
            arity: Arity::OneOrTwo,
            inverse_of: None,
            description: "logarithm",
            op: FuncImpl::Variadic(|args, _env| match args.len() {
                1 => args[0].log(None),
                2 => args[0].log(Some(&args[1])),
                _ => Err(Error::mp(messages::takes_1_or_2("log"))),
            }),
        },
        FuncDef {
            name: "ncr",
            category: C::Sundry,
            arity: Arity::Exact(2),
            inverse_of: None,
            description: "combinations",
            op: FuncImpl::Variadic(|args, _env| args[0].ncr(&args[1])),
        },
        FuncDef {
            name: "npr",
            category: C::Sundry,
            arity: Arity::Exact(2),
            inverse_of: None,
            description: "permutations",
            op: FuncImpl::Variadic(|args, _env| args[0].npr(&args[1])),
        },
        FuncDef {
            name: "cmp",
            category: C::Sundry,
            arity: Arity::Exact(2),
            inverse_of: None,
            description: "Kronecker delta",
            op: FuncImpl::Variadic(|args, _env| Ok(args[0].cmp_delta(&args[1]))),
        },
        FuncDef {
            name: "gcd",
            category: C::Sundry,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "greatest common divisor",
            op: FuncImpl::Variadic(|args, _env| numfuncs::gcd(args)),
        },
        FuncDef {
            name: "lcm",
            category: C::Sundry,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "least common multiple",
            op: FuncImpl::Variadic(|args, _env| numfuncs::lcm(args)),
        },
        // -- statistics ---------------------------------------------------------
        FuncDef {
            name: "sum",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "sum",
            op: FuncImpl::Variadic(|args, _env| Number::sum(args)),
        },
        FuncDef {
            name: "sumsq",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "sum of squares",
            op: FuncImpl::Variadic(|args, _env| Number::sumsq(args)),
        },
        FuncDef {
            name: "average",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "average",
            op: FuncImpl::Variadic(|args, _env| Number::average(args)),
        },
        FuncDef {
            name: "median",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "median",
            op: FuncImpl::Variadic(|args, _env| Number::median(args)),
        },
        FuncDef {
            name: "min",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "minimum value",
            op: FuncImpl::Variadic(|args, _env| Number::min(args)),
        },
        FuncDef {
            name: "max",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "maximum value",
            op: FuncImpl::Variadic(|args, _env| Number::max(args)),
        },
        FuncDef {
            name: "stdev",
            category: C::Statistics,
            arity: Arity::AtLeast(2),
            inverse_of: None,
            description: "sample standard deviation",
            op: FuncImpl::Variadic(|args, _env| Number::stdev(args)),
        },
        FuncDef {
            name: "stdevp",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "population standard deviation",
            op: FuncImpl::Variadic(|args, _env| Number::stdevp(args)),
        },
        FuncDef {
            name: "var",
            category: C::Statistics,
            arity: Arity::AtLeast(2),
            inverse_of: None,
            description: "sample variance",
            op: FuncImpl::Variadic(|args, _env| Number::var(args)),
        },
        FuncDef {
            name: "varp",
            category: C::Statistics,
            arity: Arity::AtLeast(1),
            inverse_of: None,
            description: "population variance",
            op: FuncImpl::Variadic(|args, _env| Number::varp(args)),
        },
    ]
}

/// True for dynamic `log<base>` subscript names (log₂…, doc 04 §1.6).
pub fn is_dynamic_log_name(name: &str) -> bool {
    if let Some(rest) = name.strip_prefix("log") {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

/// Base of a dynamic `log<base>` name.
pub fn dynamic_log_base(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("log")?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

/// Check function-name validity for definitions (doc 04 §1.7).
pub fn is_valid_function_name(name: &str) -> Result<(), String> {
    if name
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        return Err("Function name can not start with a digit".to_string());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Function name can only contain digits, letters and underscores".to_string());
    }
    Ok(())
}
