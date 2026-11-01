//! Recursive-descent parser (spec: doc 02 §5–13).
//!
//! Precedence levels (higher binds tighter), straight from doc 02 §5:
//! 1 `in` conversions · 3 additive (+ % sugar) · 4 implicit-multiply/× ·
//! 5 ÷ and mod · 6 not · 7 function application · 8 bitwise + shifts ·
//! 9 postfix % · 10 unary − / ^ / roots · 11 ! · 12 atoms (numbers,
//! variables, superscript powers).

use crate::ast::{BinOp, ConvertTarget, FuncPower, Node, Span, UnOp};
use crate::error::{messages, Error, ErrorCode};
use crate::lexer::{LexLookup, Literal, SupPower, Token, TokenKind};
use crate::number::parse_decimal_rational;
use rug::ops::Pow;
use rug::Rational;

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    lookup: &'a dyn LexLookup,
    text: &'a str,
}

/// Parse a canonical expression into an AST.
pub fn parse(tokens: Vec<Token>, text: &str, lookup: &dyn LexLookup) -> Result<Node, Error> {
    let mut p = Parser {
        tokens,
        pos: 0,
        lookup,
        text,
    };
    p.parse_statement()
}

impl<'a> Parser<'a> {
    // ------------------------------------------------------------------
    // Token plumbing
    // ------------------------------------------------------------------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_at(&self, off: usize) -> &Token {
        let idx = (self.pos + off).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if &self.tokens[self.pos].kind == kind {
            self.bump();
            true
        } else {
            false
        }
    }

    fn span_here(&self) -> Span {
        (self.peek().start, self.peek().end)
    }

    fn error_here(&self, message: &str) -> Error {
        let t = self.peek();
        Error {
            code: ErrorCode::Invalid,
            message: message.to_string(),
            token_start: Some(t.start),
            token_end: Some(t.end),
        }
    }

    fn mp_here(&self, message: &str) -> Error {
        let t = self.peek();
        Error {
            code: ErrorCode::Mp,
            message: message.to_string(),
            token_start: Some(t.start),
            token_end: Some(t.end),
        }
    }

    fn error_span(&self, message: &str, span: Span) -> Error {
        Error {
            code: ErrorCode::Invalid,
            message: message.to_string(),
            token_start: Some(span.0),
            token_end: Some(span.1),
        }
    }

    // ------------------------------------------------------------------
    // Statement level: assignment, function definition, expression
    // ------------------------------------------------------------------

    fn parse_statement(&mut self) -> Result<Node, Error> {
        if matches!(self.peek_kind(), TokenKind::EndOfInput) {
            return Err(self.error_here(messages::MALFORMED));
        }

        // `name = expr` (single assignment) or `name(params) = expr [@ desc]`.
        // A Function token can head a definition too: once `f` is defined it
        // lexes as a function name, and redefinition must still parse
        // (the evaluator rejects built-in names).
        let head_name = match self.peek_kind().clone() {
            TokenKind::Word(name) | TokenKind::Function(name) => Some(name),
            _ => None,
        };
        if let Some(name) = head_name {
            match self.peek_at(1).kind.clone() {
                TokenKind::Assign => {
                    self.bump(); // name
                    self.bump(); // =
                    let expr = self.parse_conversion()?;
                    // Only one `=` allowed (doc 02 §9: a=b=c is INVALID).
                    if matches!(self.peek_kind(), TokenKind::Assign) {
                        return Err(self.error_here(messages::MALFORMED));
                    }
                    self.expect_end()?;
                    return Ok(Node::Assign {
                        name,
                        expr: Box::new(expr),
                    });
                }
                TokenKind::LParen => {
                    // Scan to the matching close paren; a following `=`
                    // makes this a function definition (doc 02 §9).
                    if let Some(close_idx) = self.find_matching_paren(self.pos + 1) {
                        if matches!(self.tokens[close_idx + 1].kind, TokenKind::Assign) {
                            return self.parse_function_definition(name.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        let expr = self.parse_conversion()?;
        // A trailing `=` after a percentage expression is tolerated and
        // ignored (doc 02 §5.1: `(50+50)+10%=` = 110).
        self.eat(&TokenKind::Assign);
        self.expect_end()?;
        Ok(expr)
    }

    fn expect_end(&mut self) -> Result<(), Error> {
        if matches!(self.peek_kind(), TokenKind::EndOfInput) {
            Ok(())
        } else {
            Err(self.error_here(messages::MALFORMED))
        }
    }

    /// Index of the token matching the paren at `open` (same nesting depth).
    fn find_matching_paren(&self, open: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut i = open;
        while i < self.tokens.len() {
            match self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                TokenKind::EndOfInput => return None,
                _ => {}
            }
            i += 1;
        }
        None
    }

    fn parse_function_definition(&mut self, name: String) -> Result<Node, Error> {
        self.bump(); // name
        self.bump(); // (
        let mut params = Vec::new();
        if !matches!(self.peek_kind(), TokenKind::RParen) {
            loop {
                match self.peek_kind().clone() {
                    TokenKind::Word(p) => {
                        // Parameters must be purely alphabetic (doc 02 §9).
                        if !p.chars().all(|c| c.is_alphabetic()) {
                            return Err(self.error_here("Function parameters must be alphabetic"));
                        }
                        params.push(p);
                        self.bump();
                    }
                    _ => return Err(self.error_here(messages::MALFORMED)),
                }
                if self.eat(&TokenKind::Semicolon) {
                    continue;
                }
                break;
            }
        }
        if !self.eat(&TokenKind::RParen) {
            return Err(self.error_here(messages::MALFORMED));
        }
        if !self.eat(&TokenKind::Assign) {
            return Err(self.error_here(messages::MALFORMED));
        }
        let body_start = self.peek().start;
        let body = self.parse_conversion()?;
        // Body text as written, up to `@` when a description follows
        // (byte-exact for the persistence file and the edit popover).
        let body_end = if matches!(self.peek_kind(), TokenKind::At) {
            self.peek().start
        } else {
            self.text.len()
        };
        let body_source = self.text[body_start..body_end].trim().to_string();
        // Description after `@`: the raw text remainder (doc 02 §9).
        let desc = if matches!(self.peek_kind(), TokenKind::At) {
            let at_end = self.peek().end;
            let rest = self.text[at_end..].trim();
            if rest.is_empty() {
                return Err(self.error_here(messages::MALFORMED));
            }
            self.pos = self.tokens.len() - 1;
            Some(rest.to_string())
        } else {
            None
        };
        self.expect_end()?;
        Ok(Node::FuncDef {
            name,
            params,
            body: Box::new(body),
            body_source,
            desc,
        })
    }

    // ------------------------------------------------------------------
    // Level 1: conversions (lowest precedence)
    // ------------------------------------------------------------------

    fn parse_conversion(&mut self) -> Result<Node, Error> {
        // Quantity shape: Number + Unit/Currency + In → from-unit attached.
        // One leading sign belongs to the quantity (UNIT-T004: −1 c in k).
        let sign = match self.peek_kind() {
            TokenKind::Minus => Some((UnOp::Neg, self.span_here())),
            TokenKind::Plus => Some((UnOp::Plus, self.span_here())),
            _ => None,
        };
        let sign_offset = if sign.is_some() { 1 } else { 0 };
        if let TokenKind::Number(_) = self.peek_at(sign_offset).kind {
            if let (TokenKind::Unit(u) | TokenKind::Currency(u), TokenKind::In) = (
                self.peek_at(sign_offset + 1).kind.clone(),
                self.peek_at(sign_offset + 2).kind.clone(),
            ) {
                if let Some((op, span)) = sign {
                    self.bump(); // sign
                    let num_tok = self.bump(); // number
                    let unit_tok = self.bump(); // unit/currency
                    self.bump(); // in
                    let lit = Node::Literal(self.literal_of(&num_tok)?);
                    let value = Node::Unary(op, Box::new(lit)).with_span(span);
                    return self.parse_conversion_tail(
                        Box::new(value),
                        Some((u, (unit_tok.start, unit_tok.end))),
                    );
                }
                let num_tok = self.bump(); // number
                let unit_tok = self.bump(); // unit/currency
                self.bump(); // in
                let value = Node::Literal(self.literal_of(&num_tok)?);
                return self.parse_conversion_tail(
                    Box::new(value),
                    Some((u, (unit_tok.start, unit_tok.end))),
                );
            }
        }
        let expr = self.parse_additive()?;
        if matches!(self.peek_kind(), TokenKind::In) {
            self.bump();
            return self.parse_conversion_tail(Box::new(expr), None);
        }
        Ok(expr)
    }

    /// After the `in` keyword: base keyword at end-of-expression, else
    /// unit/currency target (doc 02 §8).
    fn parse_conversion_tail(
        &mut self,
        expr: Box<Node>,
        from: Option<(String, Span)>,
    ) -> Result<Node, Error> {
        // Base-representation conversion: only recognized when immediately
        // followed by end-of-expression (doc 02 §8).
        if let TokenKind::BaseKeyword(b) = self.peek_kind().clone() {
            if matches!(self.peek_at(1).kind, TokenKind::EndOfInput) {
                let span = self.span_here();
                self.bump();
                return Ok(Node::Convert {
                    expr,
                    from,
                    target: ConvertTarget::Base(b),
                    target_span: span,
                });
            }
        }
        let span = self.span_here();
        match self.peek_kind().clone() {
            TokenKind::Unit(u) => {
                self.bump();
                if from.is_some() {
                    return Ok(Node::Convert {
                        expr,
                        from,
                        target: ConvertTarget::Unit(u),
                        target_span: span,
                    });
                }
                // `x in unit2` without a from-unit is INVALID (doc 02 §8).
                Err(self.error_span(messages::MALFORMED, span))
            }
            TokenKind::Currency(c) => {
                self.bump();
                if from.is_none() {
                    return Err(self.error_span(messages::MALFORMED, span));
                }
                Ok(Node::Convert {
                    expr,
                    from,
                    target: ConvertTarget::Currency(c),
                    target_span: span,
                })
            }
            TokenKind::Word(w) => {
                // Unknown word in the target position: unit candidate →
                // UNKNOWN_UNIT (doc 02 §8).
                Err(Error {
                    code: ErrorCode::UnknownUnit,
                    message: messages::unknown_unit(&w),
                    token_start: Some(span.0),
                    token_end: Some(span.1),
                })
            }
            _ => Err(self.error_span(messages::MALFORMED, span)),
        }
    }

    fn literal_of(&self, tok: &Token) -> Result<Literal, Error> {
        match &tok.kind {
            TokenKind::Number(l) => Ok(l.clone()),
            _ => Err(self.error_here(messages::MALFORMED)),
        }
    }

    // ------------------------------------------------------------------
    // Level 3: additive with percentage sugar (doc 02 §5.1)
    // ------------------------------------------------------------------

    fn parse_additive(&mut self) -> Result<Node, Error> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_multiplicative()?;
            // Additive percentage sugar: `a ± b%` where the right side is a
            // bare percent node (decision D6; OQ-04).
            let sugar = right.is_bare_percent();
            left = Node::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                sugar,
            };
        }
        Ok(left)
    }

    // ------------------------------------------------------------------
    // Level 4: implicit multiplication and ×
    // ------------------------------------------------------------------

    fn parse_multiplicative(&mut self) -> Result<Node, Error> {
        let mut left = self.parse_divisive()?;
        loop {
            if matches!(self.peek_kind(), TokenKind::Multiply) {
                self.bump();
                let right = self.parse_divisive()?;
                left = Node::Binary {
                    op: BinOp::Mul,
                    left: Box::new(left),
                    right: Box::new(right),
                    sugar: false,
                };
            } else if self.can_implicit_multiply() {
                let right = self.parse_divisive()?;
                left = Node::Binary {
                    op: BinOp::Mul,
                    left: Box::new(left),
                    right: Box::new(right),
                    sugar: false,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn ends_value(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Number(_)
                | TokenKind::Word(_)
                | TokenKind::RParen
                | TokenKind::RRound
                | TokenKind::RFrac
                | TokenKind::FloorClose
                | TokenKind::CeilClose
        )
    }

    /// Implicit multiplication insertion points (doc 02 §12): number and
    /// variable/function/root; two grouped expressions; closing bracket and
    /// number; variable and variable/function/root; number and fraction;
    /// number and `(`.
    fn can_implicit_multiply(&self) -> bool {
        if self.pos == 0 {
            return false;
        }
        // A trailing superscript belongs to the preceding atom (x²·y, x²y):
        // the implicit-multiply pair is judged against the atom under the ².
        let mut prev_idx = self.pos - 1;
        if matches!(self.tokens[prev_idx].kind, TokenKind::Sup(_)) && prev_idx > 0 {
            prev_idx -= 1;
        }
        let prev = &self.tokens[prev_idx].kind;
        let next = self.peek_kind();
        if !Self::ends_value(prev) {
            return false;
        }
        let prev_is_number = matches!(prev, TokenKind::Number(_));
        match next {
            TokenKind::Function(_) | TokenKind::Root(_) => true,
            TokenKind::Word(_) => {
                // A word directly before `(` is a function-call attempt
                // (VAR-T003: z(2) → UNKNOWN_FUNCTION) unless it is a defined
                // single-character variable (VAR-T004: x(x+3y) multiplies).
                // Both cases attempt a call parse; the arity/unknown
                // checks downstream produce VAR-T003/T004's errors.
                true
            }
            TokenKind::LParen => true,
            TokenKind::Number(l) => {
                // number·number is INVALID (EXPR-T041); bracket/variable then
                // number is implicit multiply. Fractions only after numbers.
                if prev_is_number {
                    matches!(
                        l,
                        Literal::Fraction { .. } | Literal::WholeAndFraction { .. }
                    )
                } else {
                    !matches!(
                        l,
                        Literal::Fraction { .. }
                            | Literal::WholeAndFraction { .. }
                            | Literal::Dms { .. }
                    )
                }
            }
            _ => false,
        }
    }

    // ------------------------------------------------------------------
    // Level 5: ÷ and mod (left-assoc)
    // ------------------------------------------------------------------

    fn parse_divisive(&mut self) -> Result<Node, Error> {
        let mut left = self.parse_unary_not()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Divide => BinOp::Div,
                TokenKind::Mod => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary_not()?;
            left = Node::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                sugar: false,
            };
        }
        Ok(left)
    }

    // ------------------------------------------------------------------
    // Level 6: not (unary, right-assoc)
    // ------------------------------------------------------------------

    fn parse_unary_not(&mut self) -> Result<Node, Error> {
        if matches!(self.peek_kind(), TokenKind::Not) {
            let span = self.span_here();
            self.bump();
            let operand = self.parse_function_application()?;
            return Ok(Node::Unary(UnOp::Not, Box::new(operand)).with_span(span));
        }
        self.parse_function_application()
    }

    // ------------------------------------------------------------------
    // Level 7: function application
    // ------------------------------------------------------------------

    fn parse_function_application(&mut self) -> Result<Node, Error> {
        if let TokenKind::Function(name) = self.peek_kind().clone() {
            let span = self.span_here();
            self.bump();
            // `sin²45` — the superscript between name and argument applies to
            // the call result (FUNC-T005: sin²45 = 0.5).
            let leading_power = self.parse_func_power()?;
            // log + bare number base + bracketed argument (doc 04 §1.6):
            // `log 2 (8)` = log₂ 8.
            if name == "log" && leading_power.is_none() {
                if let TokenKind::Number(Literal::Digits {
                    int_part,
                    frac_part,
                    base,
                }) = self.peek_kind().clone()
                {
                    if frac_part.is_empty()
                        && base == 10
                        && matches!(self.peek_at(1).kind, TokenKind::LParen)
                    {
                        self.bump(); // the base number
                        let base_node = Node::Literal(Literal::Digits {
                            int_part,
                            frac_part: String::new(),
                            base: 10,
                        });
                        let mut args = self.parse_paren_args()?;
                        args.push(base_node);
                        return Ok(Node::Call {
                            name: "log".to_string(),
                            args,
                            power: None,
                            span,
                        });
                    }
                }
            }
            // Bracketed call: `sin(30)`, `log(x;y)`.
            if matches!(self.peek_kind(), TokenKind::LParen) {
                let args = self.parse_paren_args()?;
                let power = match leading_power {
                    Some(p) => Some(p),
                    None => self.parse_func_power()?,
                };
                return Ok(Node::Call {
                    name,
                    args,
                    power,
                    span,
                });
            }
            // Single argument without brackets: operand at level 8+.
            let can_take_arg = matches!(
                self.peek_kind(),
                TokenKind::Number(_)
                    | TokenKind::Word(_)
                    | TokenKind::Function(_)
                    | TokenKind::LParen
                    | TokenKind::Root(_)
                    | TokenKind::Minus
                    | TokenKind::Plus
                    | TokenKind::Not
                    | TokenKind::LRound
                    | TokenKind::LFrac
                    | TokenKind::FloorOpen
                    | TokenKind::CeilOpen
                    | TokenKind::AbsBar
            );
            if can_take_arg {
                let arg = self.parse_bitwise_level()?;
                let power = match leading_power {
                    Some(p) => Some(p),
                    None => self.parse_func_power()?,
                };
                return Ok(Node::Call {
                    name,
                    args: vec![arg],
                    power,
                    span,
                });
            }
            if leading_power.is_some() {
                return Err(self.error_here(messages::MALFORMED));
            }
            // Bare function name with no argument — variable reference
            // (errors as unknown variable at eval).
            return Ok(Node::Variable { name, span }.with_span(span));
        }
        self.parse_bitwise_level()
    }

    /// Parse `(a;b;…)` argument lists (`;` separates only at top depth).
    fn parse_paren_args(&mut self) -> Result<Vec<Node>, Error> {
        self.bump(); // (
        let mut args = Vec::new();
        if matches!(self.peek_kind(), TokenKind::RParen) {
            self.bump();
            return Ok(args);
        }
        loop {
            args.push(self.parse_conversion()?);
            if self.eat(&TokenKind::Semicolon) {
                continue;
            }
            break;
        }
        if !self.eat(&TokenKind::RParen) {
            return Err(self.error_here(messages::MALFORMED));
        }
        Ok(args)
    }

    /// `sin²` / `sin⁻¹` superscript dispatch after a function (doc 02 §5.2).
    fn parse_func_power(&mut self) -> Result<Option<FuncPower>, Error> {
        if let TokenKind::Sup(SupPower { negative, digits }) = self.peek_kind().clone() {
            self.bump();
            if negative {
                if digits == "1" {
                    return Ok(Some(FuncPower::Inverse));
                }
                return Err(self.mp_here(messages::MALFORMED));
            }
            let v: i64 = digits.parse().unwrap_or(0);
            return Ok(Some(FuncPower::Square(v)));
        }
        Ok(None)
    }

    // ------------------------------------------------------------------
    // Level 8: bitwise and shifts (left-assoc)
    // ------------------------------------------------------------------

    fn parse_bitwise_level(&mut self) -> Result<Node, Error> {
        let mut left = self.parse_percent()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::And => BinOp::And,
                TokenKind::Nand => BinOp::Nand,
                TokenKind::Or => BinOp::Or,
                TokenKind::Nor => BinOp::Nor,
                TokenKind::Xor => BinOp::Xor,
                TokenKind::Xnor => BinOp::Xnor,
                TokenKind::ShiftLeft => BinOp::ShiftLeft,
                TokenKind::ShiftRight => BinOp::ShiftRight,
                TokenKind::ShiftRightUnsigned => BinOp::ShiftRightUnsigned,
                _ => break,
            };
            self.bump();
            let right = self.parse_percent()?;
            left = Node::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                sugar: false,
            };
        }
        Ok(left)
    }

    // ------------------------------------------------------------------
    // Level 9: postfix %
    // ------------------------------------------------------------------

    fn parse_percent(&mut self) -> Result<Node, Error> {
        let mut e = self.parse_unary_power()?;
        while matches!(self.peek_kind(), TokenKind::Percent) {
            let span = self.span_here();
            self.bump();
            e = Node::Percent(Box::new(e)).with_span(span);
        }
        Ok(e)
    }

    // ------------------------------------------------------------------
    // Level 10: unary −, +, ^ (right-assoc), roots
    // ------------------------------------------------------------------

    fn parse_unary_power(&mut self) -> Result<Node, Error> {
        match self.peek_kind() {
            TokenKind::Minus => {
                let span = self.span_here();
                self.bump();
                let operand = self.parse_unary_power()?;
                return Ok(Node::Unary(UnOp::Neg, Box::new(operand)).with_span(span));
            }
            TokenKind::Plus => {
                // Unary plus before a number (doc 02 §5, EXPR-T032): allowed
                // once; `++1` is INVALID because the operand of + may not
                // itself carry a sign.
                let span = self.span_here();
                self.bump();
                let operand = self.parse_unary_power()?;
                if matches!(
                    operand,
                    Node::Unary(UnOp::Neg, _) | Node::Unary(UnOp::Plus, _)
                ) {
                    return Err(self.error_span(messages::MALFORMED, span));
                }
                return Ok(Node::Unary(UnOp::Plus, Box::new(operand)).with_span(span));
            }
            TokenKind::Root(n) => {
                let span = self.span_here();
                let idx = *n;
                self.bump();
                let operand = self.parse_unary_power()?;
                return Ok(Node::Unary(UnOp::Root(idx), Box::new(operand)).with_span(span));
            }
            _ => {}
        }
        let left = self.parse_factorial()?;
        if matches!(self.peek_kind(), TokenKind::Power) {
            self.bump();
            let right = self.parse_unary_power()?;
            return Ok(Node::Binary {
                op: BinOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
                sugar: false,
            });
        }
        Ok(left)
    }

    // ------------------------------------------------------------------
    // Level 11: factorial postfix
    // ------------------------------------------------------------------

    fn parse_factorial(&mut self) -> Result<Node, Error> {
        let mut e = self.parse_atom()?;
        // Superscript power after a group or other postfix-level atom
        // ((xy)², doc 02 §4); Number and Word atoms consume their own Sup
        // inside parse_atom.
        if let TokenKind::Sup(sp) = self.peek_kind().clone() {
            self.bump();
            let exp: i64 = sup_value(&sp)?;
            e = pow_literal(e, exp);
        }
        while matches!(self.peek_kind(), TokenKind::Factorial) {
            let span = self.span_here();
            self.bump();
            e = Node::Factorial(Box::new(e)).with_span(span);
        }
        Ok(e)
    }

    // ------------------------------------------------------------------
    // Level 12: atoms
    // ------------------------------------------------------------------

    fn parse_atom(&mut self) -> Result<Node, Error> {
        let tok = self.peek().clone();
        match tok.kind.clone() {
            TokenKind::Number(lit) => {
                self.bump();
                let mut node = Node::Literal(lit);
                // Superscript integer powers at the atom level (x², 2¹⁰).
                if let TokenKind::Sup(sp) = self.peek_kind().clone() {
                    self.bump();
                    let exp: i64 = sup_value(&sp)?;
                    node = pow_literal(node, exp);
                }
                Ok(node.with_span((tok.start, tok.end)))
            }
            TokenKind::Word(name) => {
                self.bump();
                // `name(...)` with a name that is not a defined variable is a
                // call attempt (VAR-T003 z(2), FUNC-T025 add(8) →
                // UNKNOWN_FUNCTION); a defined variable multiplies
                // (VAR-T004: x(x+3y)).
                if matches!(self.peek_kind(), TokenKind::LParen) && !self.lookup.is_variable(&name)
                {
                    let args = self.parse_paren_args()?;
                    return Ok(Node::Call {
                        name,
                        args,
                        power: None,
                        span: (tok.start, tok.end),
                    });
                }
                let mut node = self.word_node(&name, (tok.start, tok.end))?;
                if let TokenKind::Sup(sp) = self.peek_kind().clone() {
                    self.bump();
                    let exp: i64 = sup_value(&sp)?;
                    // Superscripts after a multi-character word apply to the
                    // last character only (doc 02 §4: xy² = x·y²).
                    node = apply_power_to_last_char(node, exp);
                }
                Ok(node)
            }
            TokenKind::Unit(u) => {
                // A trailing bare unit after a value is INVALID (doc 02 §8,
                // §12); a °-prefixed unit is never a variable (UNIT-T018);
                // any other bare unit at operand position is a variable.
                let prev_is_value =
                    self.pos > 0 && Self::ends_value(&self.tokens[self.pos - 1].kind);
                if prev_is_value || u.starts_with('°') || u.starts_with('˚') {
                    return Err(self.error_span(messages::MALFORMED, (tok.start, tok.end)));
                }
                self.bump();
                Ok(Node::Variable {
                    name: u,
                    span: (tok.start, tok.end),
                })
            }
            TokenKind::Currency(c) => {
                self.bump();
                Ok(Node::Variable {
                    name: c,
                    span: (tok.start, tok.end),
                })
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.parse_conversion()?;
                if !self.eat(&TokenKind::RParen) {
                    return Err(self.error_here(messages::MALFORMED));
                }
                Ok(inner)
            }
            TokenKind::LRound => {
                self.bump();
                let inner = self.parse_conversion()?;
                if !self.eat(&TokenKind::RRound) {
                    return Err(self.error_here(messages::MALFORMED));
                }
                Ok(Node::Unary(UnOp::Round, Box::new(inner)))
            }
            TokenKind::LFrac => {
                self.bump();
                let inner = self.parse_conversion()?;
                if !self.eat(&TokenKind::RFrac) {
                    return Err(self.error_here(messages::MALFORMED));
                }
                Ok(Node::Unary(UnOp::FracPart, Box::new(inner)))
            }
            TokenKind::FloorOpen => {
                self.bump();
                let inner = self.parse_conversion()?;
                if !self.eat(&TokenKind::FloorClose) {
                    return Err(self.error_here(messages::MALFORMED));
                }
                Ok(Node::Unary(UnOp::Floor, Box::new(inner)))
            }
            TokenKind::CeilOpen => {
                self.bump();
                let inner = self.parse_conversion()?;
                if !self.eat(&TokenKind::CeilClose) {
                    return Err(self.error_here(messages::MALFORMED));
                }
                Ok(Node::Unary(UnOp::Ceil, Box::new(inner)))
            }
            TokenKind::AbsBar => {
                self.bump();
                let inner = self.parse_conversion()?;
                if !self.eat(&TokenKind::AbsBar) {
                    return Err(self.error_here(messages::MALFORMED));
                }
                Ok(Node::Unary(UnOp::Abs, Box::new(inner)))
            }
            TokenKind::Root(n) => {
                self.bump();
                let operand = self.parse_unary_power()?;
                Ok(Node::Unary(UnOp::Root(n), Box::new(operand)))
            }
            TokenKind::In | TokenKind::To => Err(self.error_here(messages::MALFORMED)),
            _ => Err(self.error_here(messages::MALFORMED)),
        }
    }

    /// Build the node for a word token: a defined variable/constant name,
    /// a single-character variable, or a product of single-character
    /// variables for multi-character words (doc 02 §4).
    fn word_node(&self, name: &str, span: Span) -> Result<Node, Error> {
        if self.lookup.is_variable(name) {
            // Full-name matches win over letter products (`pi`, `rand`,
            // user-defined multi-character names).
            return Ok(Node::Variable {
                name: name.to_string(),
                span,
            });
        }
        let chars: Vec<char> = name.chars().collect();
        if chars.len() == 1 {
            return Ok(Node::Variable {
                name: name.to_string(),
                span,
            });
        }
        // Multi-character undefined words are products of their single-
        // character variables; runs of subscript digits attach to the
        // preceding character as part of its name.
        let mut parts = Vec::new();
        let mut cur = String::new();
        for c in chars {
            if crate::lexer::nd_digit_value(c).is_none()
                && crate::lexer::sub_digit_value(c).is_some()
            {
                cur.push(c);
            } else {
                if !cur.is_empty() {
                    parts.push(std::mem::take(&mut cur));
                }
                cur.push(c);
            }
        }
        if !cur.is_empty() {
            parts.push(cur);
        }
        Ok(Node::VarProduct {
            full: name.to_string(),
            parts,
            span,
        })
    }
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

fn sup_value(sp: &SupPower) -> Result<i64, Error> {
    if sp.digits.is_empty() {
        return Err(Error::mp(messages::MISSING_MINUS_DIGITS));
    }
    let v: i64 = sp.digits.parse().unwrap_or(0);
    Ok(if sp.negative { -v } else { v })
}

fn pow_literal(base: Node, exp: i64) -> Node {
    // A negative superscript is a negative exponent (2⁻¹ = 0.5), not the
    // negation of the power.
    let exp_node = if exp < 0 {
        Node::Unary(
            UnOp::Neg,
            Box::new(Node::Literal(Literal::Digits {
                int_part: exp.unsigned_abs().to_string(),
                frac_part: String::new(),
                base: 10,
            })),
        )
    } else {
        Node::Literal(Literal::Digits {
            int_part: exp.unsigned_abs().to_string(),
            frac_part: String::new(),
            base: 10,
        })
    };
    Node::Binary {
        op: BinOp::Pow,
        left: Box::new(base),
        right: Box::new(exp_node),
        sugar: false,
    }
}

/// `xy²` → x·(y²); superscript applies to the last character only.
fn apply_power_to_last_char(node: Node, exp: i64) -> Node {
    match node {
        Node::Variable { name, span } => {
            let mut cs: Vec<char> = name.chars().collect();
            let last = cs.pop().unwrap_or('x');
            let rest: String = cs.into_iter().collect();
            let powered = pow_literal(
                Node::Variable {
                    name: last.to_string(),
                    span,
                },
                exp,
            );
            if rest.is_empty() {
                powered
            } else {
                Node::Binary {
                    op: BinOp::Mul,
                    left: Box::new(Node::Variable { name: rest, span }),
                    right: Box::new(powered),
                    sugar: false,
                }
            }
        }
        Node::VarProduct {
            full,
            mut parts,
            span,
        } => {
            let last = parts.pop().unwrap_or_else(|| "x".to_string());
            let powered = pow_literal(Node::Variable { name: last, span }, exp);
            if parts.is_empty() {
                powered
            } else {
                Node::Binary {
                    op: BinOp::Mul,
                    left: Box::new(Node::VarProduct { full, parts, span }),
                    right: Box::new(powered),
                    sugar: false,
                }
            }
        }
        other => pow_literal(other, exp),
    }
}

/// Literal → exact rational (doc 03 §6).
pub fn literal_to_rational(lit: &Literal) -> Result<Rational, Error> {
    match lit {
        Literal::Digits {
            int_part,
            frac_part,
            base,
        } => {
            let digits = format!("{int_part}{frac_part}");
            if digits.is_empty() || digits.chars().all(|c| c == '0') {
                return Ok(Rational::from(0));
            }
            let num = digits_in_base_to_integer(&digits, *base)
                .ok_or_else(|| Error::mp("Malformed number"))?;
            let scale = rug::Integer::from(u64::from(*base)).pow(frac_part.len() as u32);
            Ok(Rational::from(num) / Rational::from(scale))
        }
        Literal::Fraction {
            numerator,
            denominator,
        } => Ok(Rational::from((*numerator as i64, *denominator as i64))),
        Literal::WholeAndFraction {
            whole,
            numerator,
            denominator,
        } => {
            let f = Rational::from((*numerator as i64, *denominator as i64));
            Ok(Rational::from(*whole as i64) + f)
        }
        Literal::Dms {
            degrees,
            minutes,
            seconds,
        } => {
            let mut total = parse_decimal_or_digits(degrees)?;
            if let Some(m) = minutes {
                total += parse_decimal_or_digits(m)? / Rational::from(60);
            }
            if let Some(s) = seconds {
                total += parse_decimal_or_digits(s)? / Rational::from(3600);
            }
            Ok(total)
        }
    }
}

fn parse_decimal_or_digits(s: &str) -> Result<Rational, Error> {
    parse_decimal_rational(s)
        .or_else(|| s.parse::<rug::Integer>().ok().map(Rational::from))
        .ok_or_else(|| Error::mp("Malformed number"))
}

/// Digits (ASCII 0-9a-f values) in a base → Integer.
fn digits_in_base_to_integer(digits: &str, base: u32) -> Option<rug::Integer> {
    if digits.is_empty() {
        return Some(rug::Integer::from(0));
    }
    let b = rug::Integer::from(base);
    let mut acc = rug::Integer::from(0);
    for c in digits.chars() {
        let v = c.to_digit(16)?;
        if v >= base {
            return None;
        }
        acc = acc * b.clone() + rug::Integer::from(v);
    }
    Some(acc)
}
