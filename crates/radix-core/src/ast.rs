//! Expression AST (spec: doc 02 §5–13).

use crate::lexer::Literal;

/// Binary operators (doc 02 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
    ShiftLeft,
    ShiftRight,
    ShiftRightUnsigned,
}

/// Unary operators (doc 02 §5–6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Plus,
    Not,
    /// n-th root (√, ∛, ⁿ√).
    Root(u32),
    /// `[x]` round (ties away from zero).
    Round,
    /// `{x}` fractional part (floor-based; doc 03 §4.6).
    FracPart,
    Floor,
    Ceil,
    Abs,
}

/// Function-call superscript dispatch (doc 02 §5.2): `sin²` squares the
/// result; `sin⁻¹` dispatches to the inverse function.
#[derive(Debug, Clone, PartialEq)]
pub enum FuncPower {
    /// `sin²` — raise the result to this integer power.
    Square(i64),
    /// `sin⁻¹` — inverse function.
    Inverse,
}

/// Conversion right-hand side (doc 02 §8).
#[derive(Debug, Clone, PartialEq)]
pub enum ConvertTarget {
    /// Physical unit (symbol text).
    Unit(String),
    /// Currency code.
    Currency(String),
    /// Representation base (bin/oct/dec/hex keywords).
    Base(u32),
}

/// Byte span of the originating token(s), used for error selection
/// (doc 05 §4).
pub type Span = (usize, usize);

/// A parsed expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Literal(Literal),
    /// Single variable reference (name may be multi-character).
    Variable {
        name: String,
        span: Span,
    },
    /// Product of single-character variables (`xy` = x·y), doc 02 §4;
    /// `full` keeps the whole word (a defined multi-character variable wins).
    VarProduct {
        full: String,
        parts: Vec<String>,
        span: Span,
    },
    Unary(UnOp, Box<Node>),
    Binary {
        op: BinOp,
        left: Box<Node>,
        right: Box<Node>,
        /// Additive percentage sugar (doc 02 §5.1): `a ± b%` → a·(1 ± b/100).
        sugar: bool,
    },
    /// Postfix percent `x%` = x/100.
    Percent(Box<Node>),
    /// Function application with optional superscript dispatch.
    Call {
        name: String,
        args: Vec<Node>,
        power: Option<FuncPower>,
        span: Span,
    },
    /// Factorial postfix `x!`.
    Factorial(Box<Node>),
    /// `x unit1 in unit2` / `x cur1 in cur2` / `x in bin`.
    Convert {
        expr: Box<Node>,
        from: Option<(String, Span)>,
        target: ConvertTarget,
        target_span: Span,
    },
    /// `name = expression` (doc 02 §9).
    Assign {
        name: String,
        expr: Box<Node>,
    },
    /// `name(p1;p2;…) = expression [@ description]` (doc 02 §9).
    FuncDef {
        name: String,
        params: Vec<String>,
        body: Box<Node>,
        /// Body text exactly as written (persistence file, edit popover).
        body_source: String,
        desc: Option<String>,
    },
}

impl Node {
    /// Whether this node is exactly a bare percent postfix (percentage-sugar
    /// detection, decision D6).
    pub fn is_bare_percent(&self) -> bool {
        matches!(self, Node::Percent(_))
    }

    /// Attach a span to the span-carrying variants (no-op elsewhere).
    pub fn with_span(self, span: Span) -> Node {
        match self {
            Node::Variable { name, .. } => Node::Variable { name, span },
            Node::VarProduct { full, parts, .. } => Node::VarProduct { full, parts, span },
            Node::Call {
                name, args, power, ..
            } => Node::Call {
                name,
                args,
                power,
                span,
            },
            other => other,
        }
    }
}
