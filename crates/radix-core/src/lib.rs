//! Radix calculator engine.
//!
//! Reimplementation of the documented GNOME Calculator behavior
//! (specification in `docs/`). This crate is headless: no GUI dependencies.

pub mod ast;
pub mod bitwise;
pub mod constants;
pub mod currency;
pub mod currency_fetch;
pub mod error;
pub mod eval;
pub mod financial;
pub mod functions;
pub mod lexer;
pub mod nd_table;
pub mod number;
pub mod numfuncs;
pub mod parser;
pub mod persistence;
pub mod serializer;
pub mod settings;
pub mod solve;
pub mod units;

pub use error::{Error, ErrorCode};
pub use number::Number;
pub use numfuncs::AngleUnit;
