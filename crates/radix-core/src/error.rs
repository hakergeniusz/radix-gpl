//! Error codes and user-facing messages (spec: docs 02 §10, 05 §4).

use std::fmt;

/// Engine error codes (doc 02 §10). Discriminants follow the doc's table
/// order; the integers are observable through `radix-cmd`'s
/// `Error <numeric-code>` output (decision D19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    None = 0,
    Invalid = 1,
    UnknownVariable = 2,
    UnknownFunction = 3,
    UnknownConversion = 4,
    UnknownUnit = 5,
    UnknownRate = 6,
    Mp = 7,
    TimedOut = 8,
    ResourceLimit = 9,
}

impl ErrorCode {
    /// Stable name used by the CLI contract (`Error: ErrorCode.INVALID`).
    pub fn name(&self) -> &'static str {
        match self {
            ErrorCode::None => "NONE",
            ErrorCode::Invalid => "INVALID",
            ErrorCode::UnknownVariable => "UNKNOWN_VARIABLE",
            ErrorCode::UnknownFunction => "UNKNOWN_FUNCTION",
            ErrorCode::UnknownConversion => "UNKNOWN_CONVERSION",
            ErrorCode::UnknownUnit => "UNKNOWN_UNIT",
            ErrorCode::UnknownRate => "UNKNOWN_RATE",
            ErrorCode::Mp => "MP",
            ErrorCode::TimedOut => "TIMED_OUT",
            ErrorCode::ResourceLimit => "RESOURCE_LIMIT",
        }
    }

    /// Numeric code (radix-cmd contract, doc 08 §5; decision D19).
    pub fn code_num(&self) -> u32 {
        *self as u32
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Engine error: a code plus, for message-carrying errors, the user-facing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    /// Offset/length of the offending token in the canonical expression, when known
    /// (doc 05 §4: the error selects the offending token).
    pub token_start: Option<usize>,
    pub token_end: Option<usize>,
}

impl Error {
    pub fn new(code: ErrorCode) -> Self {
        Error {
            code,
            message: String::new(),
            token_start: None,
            token_end: None,
        }
    }

    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Error {
            code,
            message: message.into(),
            token_start: None,
            token_end: None,
        }
    }

    /// Math-level error carrying its own message (`MP`).
    pub fn mp(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::Mp, message)
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::Invalid, message)
    }

    /// Status-line text per doc 05 §4: message-carrying errors show the message;
    /// syntax errors show "Malformed expression"; others fall back per code.
    pub fn status_text(&self) -> String {
        if !self.message.is_empty() {
            return self.message.clone();
        }
        match self.code {
            ErrorCode::Invalid => "Malformed expression".to_string(),
            _ => format!("Error: ErrorCode.{}", self.code),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status_text())
    }
}

impl std::error::Error for Error {}

/// Shared message strings (doc 03 §4, 04 §1, 05 §4). Centralized so the
/// localization layer (D14) can later swap them out.
pub mod messages {
    pub const MALFORMED: &str = "Malformed expression";
    pub const DIV_BY_ZERO: &str = "Division by zero is undefined";
    pub const ZERO_POW_ZERO: &str = "Zero raised to zero is undefined";
    pub const ZERO_POW_NEG: &str = "The power of zero is undefined for a negative exponent";
    pub const ZEROTH_ROOT: &str = "The zeroth root of a number is undefined";
    pub const TANGENT_UNDEF: &str =
        "Tangent is undefined for angles that are multiples of π (180°) from π∕2 (90°)";
    pub const ASIN_DOMAIN: &str = "Inverse sine is undefined for values outside [-1, 1]";
    pub const ACOS_DOMAIN: &str = "Inverse cosine is undefined for values outside [-1, 1]";
    pub const ATAN_I: &str = "Inverse tangent is undefined for values i and -i";
    pub const ACOSH_DOMAIN: &str =
        "Inverse hyperbolic cosine is undefined for values less than one";
    pub const ATANH_DOMAIN: &str =
        "Inverse hyperbolic tangent is undefined for values outside [-1, 1]";
    pub const ARG_ZERO: &str = "Argument not defined for zero";
    pub const MOD_COMPLEX: &str = "Modulus division is only defined for real numbers";
    pub const FACT_NEG_INT: &str = "Factorial is undefined for negative integers";
    pub const FACT_COMPLEX: &str = "Factorial of complex numbers is not supported";
    pub const LOG_ZERO: &str = "Logarithm of zero is undefined";
    pub const LOG_BASE: &str = "Logarithm with base 0 or 1 is undefined";
    pub const ROUND_COMPLEX: &str = "Round is only defined for real numbers";
    pub const FLOOR_COMPLEX: &str = "Floor is only defined for real numbers";
    pub const CEIL_COMPLEX: &str = "Ceiling is only defined for real numbers";
    pub const MEDIAN_COMPLEX: &str = "Median is only defined for real numbers";
    pub const MIN_COMPLEX: &str = "Minimum value is only defined for real numbers";
    pub const MAX_COMPLEX: &str = "Maximum value is only defined for real numbers";
    pub const GCD_INT: &str = "Greatest common divisor is only defined for integers";
    pub const LCM_INT: &str = "Least common multiple is only defined for integers";
    pub const COMB_NAT: &str = "Combination is only defined for non-negative integers";
    pub const COMB_N_R: &str = "Combination is undefined if n is less than r";
    pub const PERM_NAT: &str = "Permutation is only defined for non-negative integers";
    pub const PERM_N_R: &str = "Permutation is undefined if n is less than r";
    pub const BITWISE_NAT: &str = "Bitwise operation is only defined for non-negative integers";
    pub const SHIFT_INT: &str = "Shift is only possible on integers";
    pub const OVERFLOW_WORD: &str = "Overflow. Try a bigger word size";
    pub const TWOS_NAT: &str = "Two’s complement is only defined for non-negative integers";
    pub const BSWAP_INT: &str = "Swap endianness is only possible on integers";
    pub const OVERFLOW: &str = "Overflow error";
    pub const UNDERFLOW: &str = "Underflow error";
    pub const MISSING_MINUS_DIGITS: &str = "Missing digits after minus sign";
    pub const NEED_INTEGER_FACTORIZATION: &str = "Need an integer to factorize";
    pub const NO_SANE_VALUE: &str = "No sane value to store";
    pub const FIXED_FORMAT_TOO_LONG: &str =
        "The result is too long to display in fixed format. Try other result formats";

    pub fn unknown_variable(name: &str) -> String {
        format!("Unknown variable “{name}”")
    }
    pub fn unknown_function(name: &str) -> String {
        format!("Function “{name}” is not defined")
    }
    pub fn unknown_unit(name: &str) -> String {
        format!("Unknown unit “{name}”")
    }
    pub fn unknown_conversion() -> String {
        "Unknown conversion".to_string()
    }
    pub fn unknown_rate(name: &str) -> String {
        format!("Exchange rate for “{name}” is not available")
    }
    pub fn takes_n_args(name: &str, n: u32) -> String {
        format!("Function “{name}” takes {n} argument(s)")
    }
    pub fn takes_at_least(name: &str, n: u32) -> String {
        format!("Function “{name}” takes at least {n} arguments")
    }
    pub fn takes_1_or_2(name: &str) -> String {
        format!("Function “{name}” takes 1 or 2 arguments")
    }
}
