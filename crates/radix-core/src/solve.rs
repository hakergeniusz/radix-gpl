//! Top-level solve pipeline (spec: doc 02 §11, doc 05).
//!
//! Input is the *canonical* expression text (the display layer performs
//! answer-region → `_ ` replacement, separator removal and radix → `.`, doc
//! 02 §11). Handles assignments and function definitions as side effects.

use crate::currency::CurrencyTable;
use crate::error::{messages, Error, ErrorCode};
use crate::eval::{evaluate, Budget, EvalConfig, EvalEnv, EvalOutcome, EvalState, UserFunction};
use crate::functions;
use crate::lexer::{lex, LexLookup};
use crate::number::Number;
use crate::numfuncs::AngleUnit;
use crate::parser::parse;
use crate::units::UnitCatalog;

/// Outcome of solving one canonical expression.
#[derive(Debug, Clone)]
pub struct SolveOutcome {
    /// Display value (assignments display the right side; definitions 0).
    pub value: Option<Number>,
    /// Representation base for the serializer (0 = parser base).
    pub repr_base: u32,
    pub error: Option<Error>,
    /// Variable stored by `name = …` (silently skipped for constants).
    pub stored_variable: Option<String>,
    /// Function defined by `f(p;…) = …`.
    pub defined_function: Option<String>,
}

/// Parse and evaluate a standalone value string (registers file format:
/// scientific text like `5×10⁻¹`, doc 08 §2) with default base-10 config.
pub fn parse_value_string(text: &str) -> Result<Number, Error> {
    let config = EvalConfig {
        precision: 2000,
        wordlen: 64,
        angle_unit: AngleUnit::Degrees,
        base: 10,
    };
    let mut state = EvalState::default();
    let units = UnitCatalog::new();
    let currencies = CurrencyTable::new();
    let out = solve(text, &config, &mut state, &units, &currencies, None);
    match out.error {
        Some(e) => Err(e),
        None => out.value.ok_or_else(|| Error::mp(messages::MALFORMED)),
    }
}

/// Solve with a fresh state view; side effects land in `state`.
#[allow(clippy::too_many_arguments)]
pub fn solve(
    canonical: &str,
    config: &EvalConfig,
    state: &mut EvalState,
    units: &UnitCatalog,
    currencies: &CurrencyTable,
    budget: Option<&mut Budget>,
) -> SolveOutcome {
    let mut outcome = SolveOutcome {
        value: None,
        repr_base: 0,
        error: None,
        stored_variable: None,
        defined_function: None,
    };
    let lookup = StateLookup {
        state,
        units,
        currencies,
    };
    let tokens = match lex(canonical, &lookup, config.base) {
        Ok(t) => t,
        Err(e) => {
            outcome.error = Some(e);
            return outcome;
        }
    };
    if let Some(b) = budget.as_deref() {
        if canonical.chars().count() > b.max_input_chars {
            outcome.error = Some(Error::with_message(
                ErrorCode::ResourceLimit,
                "Expression is too long",
            ));
            return outcome;
        }
        if tokens.len() > b.max_tokens {
            outcome.error = Some(Error::with_message(
                ErrorCode::ResourceLimit,
                "Expression is too long",
            ));
            return outcome;
        }
    }
    let node = match parse(tokens, canonical, &lookup) {
        Ok(n) => n,
        Err(e) => {
            outcome.error = Some(e);
            return outcome;
        }
    };
    let mut env = EvalEnv {
        config: config.clone(),
        state,
        units,
        currencies,
        budget,
        scopes: Vec::new(),
        depth: 0,
    };
    match node {
        crate::ast::Node::Assign { name, expr } => match evaluate(&expr, &mut env) {
            Ok(v) => {
                // Constants are silently not overwritten (doc 02 §9).
                if !crate::constants::is_constant(&name) {
                    env.state.variables.insert(name.clone(), v.value.clone());
                    outcome.stored_variable = Some(name);
                }
                outcome.value = Some(v.value);
                outcome.repr_base = v.repr_base;
            }
            Err(e) => {
                outcome.error = Some(Error::mp(messages::NO_SANE_VALUE));
                let _ = e;
            }
        },
        crate::ast::Node::FuncDef {
            name,
            params,
            body,
            body_source,
            desc,
        } => {
            // Redefining a built-in function name fails (doc 02 §9).
            let reserved = functions::builtin_functions()
                .iter()
                .any(|f| f.name == name || f.name.to_lowercase() == name.to_lowercase())
                || functions::is_dynamic_log_name(&name);
            if reserved {
                outcome.error = Some(Error::with_message(
                    ErrorCode::Invalid,
                    "Can not use built-in function or operator names",
                ));
                return outcome;
            }
            env.state.functions.insert(
                name.clone(),
                UserFunction {
                    name: name.clone(),
                    params,
                    body: *body,
                    body_source,
                    description: desc,
                },
            );
            outcome.defined_function = Some(name);
            outcome.value = Some(Number::int(0, config.precision));
        }
        expr => match evaluate(&expr, &mut env) {
            Ok(EvalOutcome { value, repr_base }) => {
                outcome.value = Some(value.clone());
                outcome.repr_base = repr_base;
                // The answer is registered even when the value carries an
                // attached error (doc 03 §9: value displayed with error).
                if let Some(e) = &value.err {
                    outcome.error = Some(e.clone());
                }
                env.state.answer = Some(value);
            }
            Err(e) => {
                outcome.error = Some(e);
            }
        },
    }
    outcome
}

/// [`LexLookup`] over the current state.
struct StateLookup<'s> {
    state: &'s EvalState,
    units: &'s UnitCatalog,
    currencies: &'s CurrencyTable,
}

impl LexLookup for StateLookup<'_> {
    fn is_function(&self, name: &str) -> bool {
        functions::builtin_functions()
            .iter()
            .any(|f| f.name == name || f.name.to_lowercase() == name.to_lowercase())
            || functions::is_dynamic_log_name(name)
            || self.state.functions.contains_key(name)
            || self
                .state
                .functions
                .keys()
                .any(|k| k.to_lowercase() == name.to_lowercase())
    }

    fn is_currency(&self, name: &str) -> bool {
        self.currencies.is_known(name)
    }

    fn is_unit(&self, symbol: &str) -> bool {
        self.units.by_symbol(symbol).is_some()
    }

    fn is_variable(&self, name: &str) -> bool {
        self.state.variables.contains_key(name)
            || crate::constants::is_constant(name)
            || name == "_"
            || name == "rand"
    }
}

/// Convenience: evaluate a bare expression headlessly (tests, CLI).
pub fn solve_simple(
    canonical: &str,
    precision: u32,
    wordlen: u32,
    angle_unit: AngleUnit,
    base: u32,
) -> SolveOutcome {
    let mut state = EvalState::default();
    let units = UnitCatalog::new();
    let currencies = CurrencyTable::new();
    let config = EvalConfig {
        precision,
        wordlen,
        angle_unit,
        base,
    };
    solve(canonical, &config, &mut state, &units, &currencies, None)
}
