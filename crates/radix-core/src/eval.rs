//! Expression evaluation (spec: doc 03, 04, 05).
//!
//! Pure evaluation of an AST against an environment: user variables and
//! functions, built-in constants, `_` (previous answer), `rand`, unit and
//! currency conversion, word-size and angle-unit context, and the optional
//! search-provider budget (doc 05 §5).

use crate::ast::{BinOp, ConvertTarget, FuncPower, Node, UnOp};
use crate::bitwise::{self, ShiftKind};
use crate::constants;
use crate::currency::CurrencyTable;
use crate::error::{messages, Error, ErrorCode};
use crate::functions::{self, FuncCategory, FuncDef, FuncImpl, TrigFn};
use crate::lexer::LexLookup;
use crate::number::Number;
use crate::numfuncs::AngleUnit;
use crate::parser::literal_to_rational;
use crate::units::{self, UnitCatalog};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

/// Static evaluation context (settings snapshot).
#[derive(Debug, Clone)]
pub struct EvalConfig {
    /// Working precision in bits (GUI: 2000; CLI/search: 1000, decision D5).
    pub precision: u32,
    /// Word size for bitwise ops (8/16/32/64).
    pub wordlen: u32,
    pub angle_unit: AngleUnit,
    /// Parser number base (2–16).
    pub base: u32,
}

impl Default for EvalConfig {
    fn default() -> Self {
        EvalConfig {
            precision: 2000,
            wordlen: 64,
            angle_unit: AngleUnit::Degrees,
            base: 10,
        }
    }
}

/// User-defined function (doc 04 §1.7).
#[derive(Debug, Clone)]
pub struct UserFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: Node,
    /// Body text exactly as written (persistence file, edit popover).
    pub body_source: String,
    pub description: Option<String>,
}

/// Mutable evaluation state (variables, functions, previous answer).
#[derive(Debug, Clone, Default)]
pub struct EvalState {
    pub variables: HashMap<String, Number>,
    pub functions: HashMap<String, UserFunction>,
    pub answer: Option<Number>,
}

/// Evaluation result: value plus the representation base for display
/// (`10 in hex` → 10 rendered in base 16, doc 02 §8).
#[derive(Debug, Clone)]
pub struct EvalOutcome {
    pub value: Number,
    pub repr_base: u32,
}

impl EvalOutcome {
    fn plain(value: Number) -> Self {
        EvalOutcome {
            value,
            repr_base: 0,
        } // 0 = "use parser base"
    }
}

/// Search-provider evaluation budget (doc 05 §5). The GUI has none.
#[derive(Debug, Clone)]
pub struct Budget {
    pub max_input_chars: usize,
    pub max_tokens: usize,
    pub max_output_digits: usize,
    pub max_steps: u64,
    pub max_factorial: u64,
    pub max_exponent: u64,
    pub max_modexp: u64,
    pub max_permutation: u64,
    pub deadline: Option<Instant>,
    steps: u64,
}

impl Budget {
    pub fn new() -> Self {
        Budget {
            max_input_chars: 256,
            max_tokens: 256,
            max_output_digits: 256,
            max_steps: 100_000,
            max_factorial: 10_000,
            max_exponent: 100_000,
            max_modexp: 1_000_000,
            max_permutation: 10_000,
            deadline: Some(Instant::now() + std::time::Duration::from_millis(50)),
            steps: 0,
        }
    }

    fn step(&mut self) -> Result<(), Error> {
        self.steps += 1;
        if self.steps > self.max_steps {
            return Err(Error::with_message(
                ErrorCode::ResourceLimit,
                "Evaluation requires too many steps",
            ));
        }
        if let Some(d) = self.deadline {
            if Instant::now() > d {
                return Err(Error::with_message(
                    ErrorCode::TimedOut,
                    "Evaluation timed out",
                ));
            }
        }
        Ok(())
    }

    fn check(&self, cond: bool, msg: &str) -> Result<(), Error> {
        if cond {
            Ok(())
        } else {
            Err(Error::with_message(ErrorCode::ResourceLimit, msg))
        }
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything an evaluation can see.
pub struct EvalEnv<'a> {
    pub config: EvalConfig,
    pub state: &'a mut EvalState,
    pub units: &'a UnitCatalog,
    pub currencies: &'a CurrencyTable,
    pub budget: Option<&'a mut Budget>,
    /// Parameter scopes for user-function bodies (doc 04 §1.7 shadowing).
    pub scopes: Vec<HashMap<String, Number>>,
    pub depth: u32,
}

impl<'a> EvalEnv<'a> {
    /// LexLookup view over the environment (word classification).
    pub fn lookup(&self) -> EnvLookup<'_, '_> {
        EnvLookup { env: self }
    }
}

/// The [`LexLookup`] implementation backed by live evaluation state.
pub struct EnvLookup<'e, 'a> {
    env: &'e EvalEnv<'a>,
}

impl LexLookup for EnvLookup<'_, '_> {
    fn is_function(&self, name: &str) -> bool {
        if BUILTINS.with(|b| b.borrow().iter().any(|f: &FuncDefStatic| f.name == name)) {
            return true;
        }
        if functions::is_dynamic_log_name(name) {
            return true;
        }
        let lower = name.to_lowercase();
        if BUILTINS.with(|b| b.borrow().iter().any(|f: &FuncDefStatic| f.name == lower)) {
            return true;
        }
        self.env.state.functions.contains_key(name) || self.env.state.functions.contains_key(&lower)
    }

    fn is_currency(&self, name: &str) -> bool {
        self.env.currencies.has_rate(name)
    }

    fn is_unit(&self, symbol: &str) -> bool {
        self.env.units.by_symbol(symbol).is_some()
    }

    fn is_variable(&self, name: &str) -> bool {
        self.env.scopes.iter().rev().any(|s| s.contains_key(name))
            || self.env.state.variables.contains_key(name)
            || constants::is_constant(name)
            || name == "_"
            || name == "rand"
    }
}

// Built-in function table shared across lookups (thread-local cache).
type FuncDefStatic = FuncDef;
thread_local! {
    static BUILTINS: std::cell::RefCell<Vec<FuncDef>> =
        std::cell::RefCell::new(functions::builtin_functions());
}

/// Evaluate a parsed expression.
pub fn evaluate(node: &Node, env: &mut EvalEnv) -> Result<EvalOutcome, Error> {
    if let Some(b) = env.budget.as_deref_mut() {
        b.step()?;
    }
    match node {
        Node::Literal(lit) => {
            let r = literal_to_rational(lit)?;
            Ok(EvalOutcome::plain(Number::rational(
                r,
                env.config.precision,
            )))
        }
        Node::Variable { name, span } => {
            Ok(EvalOutcome::plain(variable_value(name, env, Some(*span))?))
        }
        Node::VarProduct { full, parts, span } => {
            // A defined multi-character variable wins over the product
            // (doc 02 §4). Parameters shadow globals.
            if env.scopes.iter().rev().any(|s| s.contains_key(full))
                || env.state.variables.contains_key(full)
            {
                return Ok(EvalOutcome::plain(variable_value(full, env, Some(*span))?));
            }
            let mut acc: Option<Number> = None;
            for part in parts {
                let v = variable_value(part, env, Some(*span))?;
                acc = Some(match acc {
                    None => v,
                    Some(a) => a.mul(&v),
                });
            }
            Ok(EvalOutcome::plain(acc.ok_or_else(|| {
                Error::with_message(ErrorCode::UnknownVariable, messages::unknown_variable(full))
            })?))
        }
        Node::Unary(op, inner) => {
            let v = evaluate(inner, env)?.value;
            let r = match op {
                UnOp::Neg => v.neg(),
                UnOp::Plus => v,
                UnOp::Not => bitwise::not(&v, env.config.wordlen)?,
                UnOp::Root(k) => v.root((*k).into())?,
                UnOp::Round => v.round()?,
                UnOp::FracPart => v.fractional_part()?,
                UnOp::Floor => v.floor()?,
                UnOp::Ceil => v.ceil()?,
                UnOp::Abs => v.abs(),
            };
            Ok(EvalOutcome::plain(r))
        }
        Node::Binary {
            op,
            left,
            right,
            sugar,
        } => {
            let l = evaluate(left, env)?.value;
            let r = evaluate(right, env)?.value;
            let out = binary_op(*op, &l, &r, *sugar, env)?;
            Ok(EvalOutcome::plain(out))
        }
        Node::Percent(inner) => {
            let v = evaluate(inner, env)?.value;
            Ok(EvalOutcome::plain(v.percent()?))
        }
        Node::Factorial(inner) => {
            let v = evaluate(inner, env)?.value;
            if let Some(b) = env.budget.as_deref_mut() {
                if let Some(n) = v.to_integer().and_then(|i| i.to_u64()) {
                    b.check(n <= b.max_factorial, "Factorial is too large")?;
                } else if v.is_complex() || v.re.to_f64() < 0.0 {
                    // non-integer/negative go through the normal paths
                } else {
                    return Err(Error::with_message(
                        ErrorCode::ResourceLimit,
                        "Factorial is too large",
                    ));
                }
            }
            Ok(EvalOutcome::plain(v.factorial()?))
        }
        Node::Call {
            name,
            args,
            power,
            span,
        } => call(name, args, power.as_ref(), env, *span),
        Node::Convert {
            expr,
            from,
            target,
            target_span,
        } => convert(expr, from.as_ref(), target, *target_span, env),
        Node::Assign { .. } | Node::FuncDef { .. } => {
            // Statements are handled by the solve pipeline; evaluating one
            // directly is a grammar error.
            Err(Error::with_message(ErrorCode::Invalid, messages::MALFORMED))
        }
    }
}

/// Variable resolution order: parameter scopes → user variables → `_` →
/// `rand` → built-in constants (doc 02 §4, doc 03 §7, doc 04 §2).
fn variable_value(
    name: &str,
    env: &mut EvalEnv,
    span: Option<(usize, usize)>,
) -> Result<Number, Error> {
    for scope in env.scopes.iter().rev() {
        if let Some(v) = scope.get(name) {
            return Ok(v.clone());
        }
    }
    if let Some(v) = env.state.variables.get(name) {
        return Ok(v.clone());
    }
    if name == "_" {
        if let Some(a) = env.state.answer.clone() {
            return Ok(a);
        }
        return Err(err_with_span(
            Error::with_message(ErrorCode::UnknownVariable, messages::unknown_variable("_")),
            span,
        ));
    }
    if name == "rand" {
        return Ok(crate::number::random_unit(env.config.precision));
    }
    if let Some(v) = constants::constant_value(name, env.config.precision) {
        return Ok(v);
    }
    Err(err_with_span(
        Error::with_message(ErrorCode::UnknownVariable, messages::unknown_variable(name)),
        span,
    ))
}

fn err_with_span(mut e: Error, span: Option<(usize, usize)>) -> Error {
    if let Some((s, t)) = span {
        e.token_start = Some(s);
        e.token_end = Some(t);
    }
    e
}

fn binary_op(
    op: BinOp,
    l: &Number,
    r: &Number,
    sugar: bool,
    env: &mut EvalEnv,
) -> Result<Number, Error> {
    match op {
        BinOp::Add => {
            if sugar {
                // a·(1 + b) where the right node was a bare `b%` — the right
                // value already equals b/100 (doc 02 §5.1, decision D6).
                let one = Number::one(l.prec().max(r.prec()));
                return Ok(l.mul(&one.add(r)));
            }
            Ok(l.add(r))
        }
        BinOp::Sub => {
            if sugar {
                let one = Number::one(l.prec().max(r.prec()));
                return Ok(l.mul(&one.sub(r)));
            }
            Ok(l.sub(r))
        }
        BinOp::Mul => Ok(l.mul(r)),
        BinOp::Div => l.div(r),
        BinOp::Mod => l.mod_op(r),
        BinOp::Pow => {
            if let Some(b) = env.budget.as_deref_mut() {
                if let Some(e) = r.to_integer().and_then(|i| i.to_u64()) {
                    b.check(e <= b.max_exponent, "Exponent is too large")?;
                }
            }
            l.pow_op(r)
        }
        BinOp::And => bitwise::and(l, r),
        BinOp::Nand => bitwise::nand(l, r, env.config.wordlen),
        BinOp::Or => bitwise::or(l, r),
        BinOp::Nor => bitwise::nor(l, r, env.config.wordlen),
        BinOp::Xor => bitwise::xor(l, r),
        BinOp::Xnor => bitwise::xnor(l, r, env.config.wordlen),
        BinOp::ShiftLeft => bitwise::shift(l, r, env.config.wordlen, ShiftKind::Left),
        BinOp::ShiftRight => bitwise::shift(l, r, env.config.wordlen, ShiftKind::Right),
        BinOp::ShiftRightUnsigned => {
            bitwise::shift(l, r, env.config.wordlen, ShiftKind::UnsignedRight)
        }
    }
}

/// Function dispatch: built-ins (case-insensitive), dynamic `log<base>`,
/// then user functions (doc 04 §1).
fn call(
    name: &str,
    args: &[Node],
    power: Option<&FuncPower>,
    env: &mut EvalEnv,
    span: (usize, usize),
) -> Result<EvalOutcome, Error> {
    // Evaluate arguments first (argument errors take precedence).
    let mut vals = Vec::new();
    for a in args {
        vals.push(evaluate(a, env)?.value);
    }

    // Built-in (exact match, then lowercase; doc 02 §4).
    let builtin = BUILTINS.with(|b| {
        let table = b.borrow();
        table.iter().find(|f| f.name == name).cloned().or_else(|| {
            let lower = name.to_lowercase();
            table.iter().find(|f| f.name == lower).cloned()
        })
    });
    let def: Option<(String, FuncDef)> = builtin.map(|d| (String::new(), d)).or_else(|| {
        // Dynamic log₂-style names (doc 04 §1.6).
        if functions::is_dynamic_log_name(name) {
            let base = functions::dynamic_log_base(name)?;
            let def = BUILTINS.with(|b| b.borrow().iter().find(|f| f.name == "log").cloned())?;
            return Some((base.to_string(), def));
        }
        None
    });

    if let Some((log_base, def)) = def {
        match power {
            Some(FuncPower::Inverse) => {
                // Dispatch to the inverse of the called function
                // (cos⁻¹ → cos⁻¹/acos entry, doc 04 §1.1).
                let inv_name = format!("{name}⁻¹");
                let inv = BUILTINS.with(|b| {
                    b.borrow()
                        .iter()
                        .find(|f| f.name == inv_name || f.name == inverse_alias(name))
                        .cloned()
                });
                return match inv {
                    Some(inv) => run_def(&inv, vals, None, env, span).map(EvalOutcome::plain),
                    None => Err(err_with_span(
                        Error::with_message(
                            ErrorCode::UnknownFunction,
                            messages::unknown_function(name),
                        ),
                        Some(span),
                    )),
                };
            }
            Some(FuncPower::Square(n)) => {
                // sin²45 = (sin 45)² (doc 02 §5.2).
                let log_base = if log_base.is_empty() {
                    None
                } else {
                    Some(log_base)
                };
                let v = run_def(&def, vals, log_base, env, span)?;
                let powered = v.pow_op(&Number::int(*n, v.prec()))?;
                return Ok(EvalOutcome::plain(powered));
            }
            None => {}
        }
        let log_base = if log_base.is_empty() {
            None
        } else {
            Some(log_base)
        };
        return run_def(&def, vals, log_base, env, span).map(EvalOutcome::plain);
    }

    // User functions (doc 04 §1.7): case-insensitive.
    let key = env
        .state
        .functions
        .keys()
        .find(|k| k.as_str() == name || k.to_lowercase() == name.to_lowercase())
        .cloned();
    if let Some(key) = key {
        let f = env.state.functions[&key].clone();
        if f.params.len() != vals.len() {
            return Err(err_with_span(
                Error::mp(messages::takes_n_args(&f.name, f.params.len() as u32)),
                Some(span),
            ));
        }
        if let Some(b) = env.budget.as_deref_mut() {
            b.step()?;
        }
        env.depth += 1;
        if env.depth > 512 {
            env.depth -= 1;
            return Err(Error::with_message(
                ErrorCode::ResourceLimit,
                "Evaluation requires too many steps",
            ));
        }
        let mut scope = HashMap::new();
        for (p, v) in f.params.iter().zip(vals) {
            scope.insert(p.clone(), v);
        }
        env.scopes.push(scope);
        let out = evaluate(&f.body, env);
        env.scopes.pop();
        env.depth -= 1;
        return match (power, out) {
            (Some(FuncPower::Square(n)), Ok(o)) => {
                let v = o.value.pow_op(&Number::int(*n, o.value.prec()))?;
                Ok(EvalOutcome {
                    value: v,
                    repr_base: o.repr_base,
                })
            }
            (_, o) => o,
        };
    }

    Err(err_with_span(
        Error::with_message(ErrorCode::UnknownFunction, messages::unknown_function(name)),
        Some(span),
    ))
}

fn inverse_alias(base: &str) -> &'static str {
    match base {
        "sin" => "asin",
        "cos" => "acos",
        "tan" => "atan",
        "sinh" => "asinh",
        "cosh" => "acosh",
        "tanh" => "atanh",
        _ => "",
    }
}

fn run_def(
    def: &FuncDef,
    vals: Vec<Number>,
    log_base: Option<String>,
    env: &mut EvalEnv,
    span: (usize, usize),
) -> Result<Number, Error> {
    // Arity check with the documented messages (doc 04 §1).
    let n = vals.len() as u32;
    match def.arity {
        functions::Arity::Exact(k) => {
            if n != k {
                return Err(err_with_span(
                    Error::mp(messages::takes_n_args(def.name, k)),
                    Some(span),
                ));
            }
        }
        functions::Arity::OneOrTwo => {
            if n != 1 && n != 2 {
                return Err(err_with_span(
                    Error::mp(messages::takes_1_or_2(def.name)),
                    Some(span),
                ));
            }
        }
        functions::Arity::AtLeast(k) => {
            if n < k {
                return Err(err_with_span(
                    Error::mp(messages::takes_at_least(def.name, k)),
                    Some(span),
                ));
            }
        }
    }
    // Dynamic log base: appended as the last argument (doc 04 §1.6).
    let mut vals = vals;
    if let Some(base) = log_base {
        vals.push(Number::int(
            base.parse::<i64>().unwrap_or(0),
            env.config.precision,
        ));
    }
    let func_env = functions::Env {
        wordlen: env.config.wordlen,
        angle_unit: env.config.angle_unit,
        precision: env.config.precision,
        call: &|name, args| {
            // Nested built-in calls from variadic impls (unused today).
            let _ = (name, args);
            Err(Error::mp("unsupported"))
        },
    };
    match &def.op {
        FuncImpl::Trig(t) => {
            let x = &vals[0];
            let r = match t {
                TrigFn::Sin => x.sin(env.config.angle_unit)?,
                TrigFn::Cos => x.cos(env.config.angle_unit)?,
                TrigFn::Tan => x.tan(env.config.angle_unit)?,
                TrigFn::Asin => x.asin(env.config.angle_unit)?,
                TrigFn::Acos => x.acos(env.config.angle_unit)?,
                TrigFn::Atan => x.atan(env.config.angle_unit)?,
                TrigFn::Arg => x.arg(env.config.angle_unit)?,
            };
            Ok(r)
        }
        FuncImpl::Unary(f) => f(&vals[0], &func_env),
        FuncImpl::Variadic(f) => f(&vals, &func_env),
    }
}

/// Conversions (doc 02 §8): units require a from-unit; currencies require a
/// from-currency; base keywords set the representation base.
fn convert(
    expr: &Node,
    from: Option<&(String, (usize, usize))>,
    target: &ConvertTarget,
    target_span: (usize, usize),
    env: &mut EvalEnv,
) -> Result<EvalOutcome, Error> {
    let value = evaluate(expr, env)?.value;
    match target {
        ConvertTarget::Base(b) => Ok(EvalOutcome {
            value,
            repr_base: *b,
        }),
        ConvertTarget::Unit(to_sym) => {
            let to = env.units.by_symbol(to_sym).ok_or_else(|| {
                err_with_span(
                    Error::with_message(ErrorCode::UnknownUnit, messages::unknown_unit(to_sym)),
                    Some(target_span),
                )
            })?;
            let (from_sym, from_span) =
                from.ok_or_else(|| Error::with_message(ErrorCode::Invalid, messages::MALFORMED))?;
            let from_unit = env.units.by_symbol(from_sym).ok_or_else(|| {
                err_with_span(
                    Error::with_message(ErrorCode::UnknownUnit, messages::unknown_unit(from_sym)),
                    Some(*from_span),
                )
            })?;
            let v = units::convert(&value, from_unit, to, env.config.precision)?;
            Ok(EvalOutcome::plain(v))
        }
        ConvertTarget::Currency(to_code) => {
            let (from_code, _) =
                from.ok_or_else(|| Error::with_message(ErrorCode::Invalid, messages::MALFORMED))?;
            let v = env
                .currencies
                .convert(&value, from_code, to_code, env.config.precision)?;
            Ok(EvalOutcome::plain(v))
        }
    }
}

/// Category of a built-in function for UI grouping (doc 04 §1.7).
pub fn builtin_category(name: &str) -> Option<FuncCategory> {
    BUILTINS.with(|b| {
        b.borrow()
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.category)
    })
}

/// Rc helper for future sharing of immutable state.
pub type SharedState = Rc<()>;
