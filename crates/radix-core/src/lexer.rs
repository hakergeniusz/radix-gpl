//! Expression lexer (spec: doc 02 §2–4, §11).
//!
//! The lexer classifies characters into tokens, recognizes numeric literals
//! (bases, prefixes, suffixes, vulgar fractions, sexagesimal DMS), and
//! classifies words via the lookup context (operator words, functions,
//! currencies, units, variables). Unit/currency tokens are reclassified as
//! variables unless adjacent to the conversion keyword (handled here via a
//! post-pass, doc 02 §4).

use crate::error::{messages, Error, ErrorCode};
use crate::nd_table::ND_RANGES;

/// Lookup context used for word classification (doc 02 §4 priority order).
pub trait LexLookup {
    /// Built-in or user-defined function name (case-insensitive).
    fn is_function(&self, name: &str) -> bool;
    /// Currency known *and* currently holding a rate.
    fn is_currency(&self, name: &str) -> bool;
    /// Unit symbol exists (ambiguity rule already applied: multi-match → false).
    fn is_unit(&self, symbol: &str) -> bool;
    /// Variable name currently defined (doc 02 §4 word+paren disambiguation).
    fn is_variable(&self, name: &str) -> bool;
    /// Localized conversion keywords (doc 02 §8); English defaults.
    fn keyword_in(&self) -> &str {
        "in"
    }
    fn keyword_to(&self) -> &str {
        "to"
    }
}

/// The default English/C-locale lookup used when no registries are wired yet.
pub struct EmptyLookup;
impl LexLookup for EmptyLookup {
    fn is_function(&self, _: &str) -> bool {
        false
    }
    fn is_currency(&self, _: &str) -> bool {
        false
    }
    fn is_unit(&self, _: &str) -> bool {
        false
    }
    fn is_variable(&self, _: &str) -> bool {
        false
    }
}

/// Numeric literal as scanned (exact components; evaluation builds the value).
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer part, optional fraction digits, in `base` (2–16).
    Digits {
        int_part: String,
        frac_part: String,
        base: u32,
    },
    /// Vulgar fraction n/d (Unicode fraction character).
    Fraction { numerator: u32, denominator: u32 },
    /// `1½` — whole part plus fraction, base 10.
    WholeAndFraction {
        whole: u64,
        numerator: u32,
        denominator: u32,
    },
    /// Sexagesimal `d°[m[′][s[″]]]` (doc 02 §3.7).
    Dms {
        degrees: String,
        minutes: Option<String>,
        seconds: Option<String>,
    },
}

/// Superscript run: optional leading ⁻ then digits (`²`, `⁻¹`, `¹⁰`).
#[derive(Debug, Clone, PartialEq)]
pub struct SupPower {
    pub negative: bool,
    pub digits: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(Literal),
    /// Word classified as variable (may be multi-character, may carry a
    /// trailing subscript that is part of the name).
    Word(String),
    /// Function name as recognized by the lookup (name may include a
    /// subscript base, e.g. `log₂`).
    Function(String),
    /// Unit symbol (possibly with superscript suffix, e.g. `m²`).
    Unit(String),
    /// Currency code with a loaded rate.
    Currency(String),
    /// Conversion keyword (`in` / `to`, translated).
    In,
    To,
    /// Base-representation keyword after `in` (bin/oct/dec/hex + long forms).
    BaseKeyword(u32),
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    Factorial,
    Percent,
    Mod,
    Assign,
    Semicolon,
    At,
    LParen,
    RParen,
    /// `[` round-to-nearest, `]` close.
    LRound,
    RRound,
    /// `{` fractional part, `}` close.
    LFrac,
    RFrac,
    FloorOpen,
    FloorClose,
    CeilOpen,
    CeilClose,
    AbsBar,
    /// √ / ∛ / ∜ or superscript/subscript-digit √ (index 2/3/4/n).
    Root(u32),
    /// Postfix superscript power (², ³, ⁻¹, ¹⁰…).
    Sup(SupPower),
    /// Subscript digits (base suffix after numbers, part of word names,
    /// root index before √, `log₂` after function words).
    Sub(String),
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
    Not,
    ShiftLeft,
    ShiftRight,
    ShiftRightUnsigned,
    /// `×10` exponent insertion is expressed as multiply+number+superscript
    /// in the source text; no dedicated token.
    EndOfInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    /// Byte offsets into the canonical input (doc 05 §4 error selection).
    pub start: usize,
    pub end: usize,
}

impl Token {
    fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Token { kind, start, end }
    }
}

/// Unicode Nd digit value (doc 02 §2: any Unicode decimal digit).
pub fn nd_digit_value(c: char) -> Option<u32> {
    let cp = c as u32;
    for &(lo, hi, off) in ND_RANGES {
        if cp >= lo && cp <= hi {
            return Some(cp - off);
        }
        if cp < lo {
            break;
        }
    }
    None
}

/// Hex digit value including A–F (and fullwidth forms via Nd table).
pub fn hex_digit_value(c: char) -> Option<u32> {
    if let Some(d) = nd_digit_value(c) {
        return Some(d);
    }
    match c {
        'a' | 'A' => Some(10),
        'b' | 'B' => Some(11),
        'c' | 'C' => Some(12),
        'd' | 'D' => Some(13),
        'e' | 'E' => Some(14),
        'f' | 'F' => Some(15),
        _ => None,
    }
}

const SUP_DIGITS: &[(char, u32)] = &[
    ('⁰', 0),
    ('¹', 1),
    ('²', 2),
    ('³', 3),
    ('⁴', 4),
    ('⁵', 5),
    ('⁶', 6),
    ('⁷', 7),
    ('⁸', 8),
    ('⁹', 9),
];
const SUP_MINUS: char = '⁻';
const SUB_DIGITS: &[(char, u32)] = &[
    ('₀', 0),
    ('₁', 1),
    ('₂', 2),
    ('₃', 3),
    ('₄', 4),
    ('₅', 5),
    ('₆', 6),
    ('₇', 7),
    ('₈', 8),
    ('₉', 9),
];

const FRACTIONS: &[(char, u32, u32)] = &[
    ('½', 1, 2),
    ('⅓', 1, 3),
    ('⅔', 2, 3),
    ('¼', 1, 4),
    ('¾', 3, 4),
    ('⅕', 1, 5),
    ('⅖', 2, 5),
    ('⅗', 3, 5),
    ('⅘', 4, 5),
    ('⅙', 1, 6),
    ('⅚', 5, 6),
    ('⅛', 1, 8),
    ('⅜', 3, 8),
    ('⅝', 5, 8),
    ('⅞', 7, 8),
];

const DEGREE_CHARS: &[char] = &['°', '˚'];
const MINUTE_CHARS: &[char] = &['\'', '′', '’'];
const SECOND_CHARS: &[char] = &['"', '″', '”'];

fn sup_digit(c: char) -> Option<u32> {
    SUP_DIGITS.iter().find(|(ch, _)| *ch == c).map(|(_, v)| *v)
}
/// Superscript digit character for a value (² is U+00B2, not U+2070+2).
fn sup_char(v: u32) -> char {
    SUP_DIGITS
        .iter()
        .find(|(_, val)| *val == v)
        .map(|(ch, _)| *ch)
        .unwrap_or('?')
}
fn sub_digit(c: char) -> Option<u32> {
    SUB_DIGITS.iter().find(|(ch, _)| *ch == c).map(|(_, v)| *v)
}

/// Public subscript-digit value (parser uses it for word splitting).
pub fn sub_digit_value(c: char) -> Option<u32> {
    sub_digit(c)
}
fn fraction_char(c: char) -> Option<(u32, u32)> {
    FRACTIONS
        .iter()
        .find(|(ch, _, _)| *ch == c)
        .map(|(_, n, d)| (*n, *d))
}

/// Lex the canonical expression text (doc 02 §11: already canonicalized —
/// radix `.`, no thousands separators).
pub fn lex(input: &str, lookup: &dyn LexLookup, parser_base: u32) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    // Byte offset of char index (for token positions).
    let byte_at = |idx: usize| -> usize { chars[..idx].iter().map(|c| c.len_utf8()).sum() };

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start_byte = byte_at(i);

        // ---- words (letters, _, \, with trailing subscripts) ------------
        if c.is_alphabetic() || c == '_' || c == '\\' {
            let (tok, next) = scan_word(&chars, i, start_byte, &byte_at, lookup, parser_base)?;
            tokens.push(tok);
            i = next;
            continue;
        }

        // ---- numbers (digits, radix point, prefixes, fractions, DMS) ----
        if nd_digit_value(c).is_some() || c == '.' || (c == '0' && is_prefix_start(&chars, i)) {
            let (tok, next) = scan_number(&chars, i, start_byte, &byte_at, parser_base)?;
            tokens.push(tok);
            i = next;
            continue;
        }

        // ---- superscript / subscript runs -------------------------------
        if sup_digit(c).is_some() || c == SUP_MINUS {
            let (tok, next) = scan_sup_run(&chars, i, start_byte, &byte_at)?;
            tokens.push(tok);
            i = next;
            continue;
        }
        if sub_digit(c).is_some() {
            let (run, next) = scan_sub_run(&chars, i);
            let end_byte = byte_at(next);
            // Subscript digits immediately followed by √ are a root index.
            if next < chars.len() && chars[next] == '√' {
                let n = sub_value(&run).ok_or_else(|| Error::invalid(messages::MALFORMED))?;
                tokens.push(Token::new(
                    TokenKind::Root(n),
                    start_byte,
                    byte_at(next + 1),
                ));
                i = next + 1;
            } else {
                tokens.push(Token::new(TokenKind::Sub(run), start_byte, end_byte));
                i = next;
            }
            continue;
        }

        // ---- vulgar fractions -------------------------------------------
        if let Some((n, d)) = fraction_char(c) {
            tokens.push(Token::new(
                TokenKind::Number(Literal::Fraction {
                    numerator: n,
                    denominator: d,
                }),
                start_byte,
                start_byte + c.len_utf8(),
            ));
            i += 1;
            continue;
        }

        // ---- DMS continuation after a number is handled in scan_number;
        // a ° here starts a °-unit word (e.g. °C) or is an error. ---------
        if DEGREE_CHARS.contains(&c) {
            let (tok, next) = scan_degree_unit(&chars, i, start_byte, &byte_at)?;
            tokens.push(tok);
            i = next;
            continue;
        }

        // ---- operators and punctuation ----------------------------------
        let (kind, len) = match c {
            '+' => (TokenKind::Plus, 1),
            '-' | '−' | '–' => (TokenKind::Minus, 1),
            '*' | '×' => (TokenKind::Multiply, 1),
            '/' | '∕' | '÷' => (TokenKind::Divide, 1),
            '^' => (TokenKind::Power, 1),
            '!' => (TokenKind::Factorial, 1),
            '%' => (TokenKind::Percent, 1),
            '=' => (TokenKind::Assign, 1),
            ';' => (TokenKind::Semicolon, 1),
            '@' => (TokenKind::At, 1),
            '(' => (TokenKind::LParen, 1),
            ')' => (TokenKind::RParen, 1),
            '[' => (TokenKind::LRound, 1),
            ']' => (TokenKind::RRound, 1),
            '{' => (TokenKind::LFrac, 1),
            '}' => (TokenKind::RFrac, 1),
            '⌊' => (TokenKind::FloorOpen, 1),
            '⌋' => (TokenKind::FloorClose, 1),
            '⌈' => (TokenKind::CeilOpen, 1),
            '⌉' => (TokenKind::CeilClose, 1),
            '|' => (TokenKind::AbsBar, 1),
            '√' => (TokenKind::Root(2), 1),
            '∛' => (TokenKind::Root(3), 1),
            '∜' => (TokenKind::Root(4), 1),
            '∧' => (TokenKind::And, 1),
            '⊼' => (TokenKind::Nand, 1),
            '∨' => (TokenKind::Or, 1),
            '⊽' => (TokenKind::Nor, 1),
            '⊻' | '⊕' => (TokenKind::Xor, 1),
            '⊙' => (TokenKind::Xnor, 1),
            '¬' | '~' => (TokenKind::Not, 1),
            '«' | '≪' => (TokenKind::ShiftLeft, 1),
            '»' | '≫' => (TokenKind::ShiftRight, 1),
            '⋙' => (TokenKind::ShiftRightUnsigned, 1),
            _ => {
                return Err(Error {
                    code: ErrorCode::Invalid,
                    message: messages::MALFORMED.to_string(),
                    token_start: Some(start_byte),
                    token_end: Some(start_byte + c.len_utf8()),
                });
            }
        };
        tokens.push(Token::new(kind, start_byte, start_byte + c.len_utf8()));
        i += len;
    }

    let end = input.len();
    tokens.push(Token::new(TokenKind::EndOfInput, end, end));
    reclassify_units(&mut tokens);
    Ok(tokens)
}

fn is_prefix_start(chars: &[char], i: usize) -> bool {
    chars[i] == '0'
        && matches!(
            chars.get(i + 1),
            Some('b') | Some('B') | Some('o') | Some('O') | Some('x') | Some('X')
        )
}

/// Scan a word: alphabetic run (+ `_`, `\`) with optional trailing subscript
/// digits; classify per doc 02 §4.
#[allow(clippy::too_many_arguments)]
fn scan_word(
    chars: &[char],
    start: usize,
    start_byte: usize,
    byte_at: &dyn Fn(usize) -> usize,
    lookup: &dyn LexLookup,
    parser_base: u32,
) -> Result<(Token, usize), Error> {
    let mut i = start;
    while i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == '\\') {
        i += 1;
    }
    // Hex-letter words in base 16 that form a valid hex literal are numbers
    // (doc 02 §2: a–f are hex digits when part of a number). Only when the
    // word is entirely hex digits and stops at a non-word char.
    if parser_base == 16 {
        let all_hex = chars[start..i]
            .iter()
            .all(|c| hex_digit_value(*c).is_some());
        let stops =
            i >= chars.len() || !(chars[i].is_alphabetic() || sub_digit(chars[i]).is_some());
        if all_hex && stops {
            let text: String = chars[start..i].iter().collect();
            return Ok((
                Token::new(
                    TokenKind::Number(Literal::Digits {
                        int_part: text,
                        frac_part: String::new(),
                        base: 16,
                    }),
                    start_byte,
                    byte_at(i),
                ),
                i,
            ));
        }
    }
    // Trailing subscript digits are part of the word (doc 02 §4).
    let mut sub = String::new();
    while i < chars.len() {
        if let Some(v) = sub_digit(chars[i]) {
            sub.push(char::from_digit(v, 16).unwrap());
            i += 1;
        } else {
            break;
        }
    }
    let mut text: String = chars[start..i - sub.chars().count()].iter().collect();

    // Word + base suffix forming a literal (`A₁₆` = 10 in any parser base,
    // LIT-T001; `A₂` falls through because A is not a base-2 digit).
    if let Some(sfx) = sub_value(&sub) {
        if (2..=16).contains(&sfx)
            && !text.is_empty()
            && text
                .chars()
                .all(|c| hex_digit_value(c).is_some_and(|v| v < sfx))
        {
            return Ok((
                Token::new(
                    TokenKind::Number(Literal::Digits {
                        int_part: text,
                        frac_part: String::new(),
                        base: sfx,
                    }),
                    start_byte,
                    byte_at(i),
                ),
                i,
            ));
        }
    }

    // Operator words (case-insensitive on the lowercased word, doc 02 §4).
    let lower: String = text.to_lowercase();
    let op_kind = match lower.as_str() {
        "mod" => Some(TokenKind::Mod),
        "and" => Some(TokenKind::And),
        "nand" => Some(TokenKind::Nand),
        "or" => Some(TokenKind::Or),
        "nor" => Some(TokenKind::Nor),
        "xor" => Some(TokenKind::Xor),
        "xnor" | "nxor" => Some(TokenKind::Xnor),
        "not" => Some(TokenKind::Not),
        "\\cdot" => Some(TokenKind::Multiply),
        _ => None,
    };
    if let Some(kind) = op_kind {
        if sub.is_empty() {
            return Ok((Token::new(kind, start_byte, byte_at(i)), i));
        }
    }
    // Conversion keywords are matched against the translation (doc 02 §8).
    if sub.is_empty() && (text == lookup.keyword_in() || lower == lookup.keyword_in()) {
        return Ok((Token::new(TokenKind::In, start_byte, byte_at(i)), i));
    }
    if sub.is_empty() && (text == lookup.keyword_to() || lower == lookup.keyword_to()) {
        return Ok((Token::new(TokenKind::To, start_byte, byte_at(i)), i));
    }

    // Base-representation keywords (doc 02 §8); the parser enforces the
    // end-of-expression condition.
    if sub.is_empty() {
        let base = match lower.as_str() {
            "bin" | "binary" => Some(2u32),
            "oct" | "octal" => Some(8),
            "dec" | "decimal" => Some(10),
            "hex" | "hexadecimal" => Some(16),
            _ => None,
        };
        if let Some(b) = base {
            return Ok((
                Token::new(TokenKind::BaseKeyword(b), start_byte, byte_at(i)),
                i,
            ));
        }
    }

    // Function lookup: case-insensitive (exact first, then lowercase), with a
    // subscript suffix forming a dynamic name (log₂, doc 04 §1.6) and a
    // superscript suffix forming an inverse (cos⁻¹, doc 04 §1.1).
    let with_sub = format!("{text}{sub}");
    let candidate = |name: &str| -> bool {
        lookup.is_function(name) || lookup.is_function(&name.to_lowercase())
    };
    // Superscript suffix: consumed only when it completes a function name,
    // so `sin²45` stays Function(sin) + Sup(2) (application power).
    if i < chars.len() && (chars[i] == SUP_MINUS || sup_digit(chars[i]).is_some()) {
        let mut j = i;
        let mut ext = String::new();
        if chars[j] == SUP_MINUS {
            ext.push('⁻');
            j += 1;
        }
        while j < chars.len() {
            match sup_digit(chars[j]) {
                Some(v) => {
                    ext.push(char::from_digit(v, 16).unwrap());
                    j += 1;
                }
                None => break,
            }
        }
        if !ext.is_empty() {
            let with_sup = format!("{text}{ext}");
            if candidate(&with_sup) {
                return Ok((
                    Token::new(TokenKind::Function(with_sup), start_byte, byte_at(j)),
                    j,
                ));
            }
        }
    }
    if !sub.is_empty() && candidate(&with_sub) {
        return Ok((
            Token::new(TokenKind::Function(with_sub), start_byte, byte_at(i)),
            i,
        ));
    }
    if sub.is_empty() && candidate(&text) {
        return Ok((
            Token::new(TokenKind::Function(text), start_byte, byte_at(i)),
            i,
        ));
    }

    // A word directly followed by `(` is a function-call attempt or a
    // definition name (doc 02 §9: `name(arg1;arg2;…)`), never a unit or
    // currency — even when the text matches one (VAR-T007: `f(x;y;z)=…`
    // where f is also farad).
    if i < chars.len() && chars[i] == '(' {
        text.push_str(&sub);
        return Ok((Token::new(TokenKind::Word(text), start_byte, byte_at(i)), i));
    }

    // Currency (with a loaded rate, doc 02 §4).
    if lookup.is_currency(&text) {
        return Ok((
            Token::new(TokenKind::Currency(text), start_byte, byte_at(i)),
            i,
        ));
    }

    // Units, optionally with a superscript suffix (m², doc 02 §4).
    if i < chars.len() {
        if let Some(v) = sup_digit(chars[i]) {
            let sup_text = format!("{text}{}", sup_char(v));
            if lookup.is_unit(&sup_text) {
                let sup_end = i + 1;
                return Ok((
                    Token::new(TokenKind::Unit(sup_text), start_byte, byte_at(sup_end)),
                    sup_end,
                ));
            }
        }
    }
    if lookup.is_unit(&text) {
        return Ok((Token::new(TokenKind::Unit(text), start_byte, byte_at(i)), i));
    }
    // Case-insensitive unit pass (doc 02 §4: case-sensitive first, then
    // case-insensitive; ambiguity resolves to no unit inside is_unit).
    let upper_first = {
        let mut cs = text.chars();
        match cs.next() {
            Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
            None => String::new(),
        }
    };
    if lookup.is_unit(&lower) {
        return Ok((
            Token::new(TokenKind::Unit(lower), start_byte, byte_at(i)),
            i,
        ));
    }
    if lookup.is_unit(&upper_first) {
        return Ok((
            Token::new(TokenKind::Unit(upper_first), start_byte, byte_at(i)),
            i,
        ));
    }

    // Plain variable word (with any subscript suffix as part of the name).
    text.push_str(&sub);
    Ok((Token::new(TokenKind::Word(text), start_byte, byte_at(i)), i))
}

/// °-prefixed unit word (°C, ˚F) — never reclassified (doc 02 §4).
fn scan_degree_unit(
    chars: &[char],
    start: usize,
    start_byte: usize,
    byte_at: &dyn Fn(usize) -> usize,
) -> Result<(Token, usize), Error> {
    let mut i = start + 1;
    while i < chars.len() && (chars[i].is_alphabetic()) {
        i += 1;
    }
    if i == start + 1 {
        // Bare ° — invalid (DMS continuation is handled in scan_number).
        return Err(Error {
            code: ErrorCode::Invalid,
            message: messages::MALFORMED.to_string(),
            token_start: Some(start_byte),
            token_end: Some(byte_at(i)),
        });
    }
    let text: String = chars[start..i].iter().collect();
    Ok((Token::new(TokenKind::Unit(text), start_byte, byte_at(i)), i))
}

/// Post-lex reclassification (doc 02 §4): in the from-unit position, an
/// unknown word directly after a value and directly before `in` becomes a
/// unit candidate (UNIT-T014: `5 x in y` errors as UNKNOWN_UNIT, decision
/// D7). Unit/currency tokens keep their kind elsewhere — the parser decides
/// between variable use (bare unit word), INVALID (trailing unit, °-prefixed
/// bare unit) and conversion shapes.
fn reclassify_units(tokens: &mut [Token]) {
    let n = tokens.len();
    let ends_value = |k: &TokenKind| {
        matches!(
            k,
            TokenKind::Number(_)
                | TokenKind::Word(_)
                | TokenKind::RParen
                | TokenKind::RRound
                | TokenKind::RFrac
                | TokenKind::FloorClose
                | TokenKind::CeilClose
        )
    };
    for idx in 0..n {
        let next_is_in =
            idx + 1 < n && matches!(tokens[idx + 1].kind, TokenKind::In | TokenKind::To);
        if next_is_in {
            if let TokenKind::Word(w) = tokens[idx].kind.clone() {
                if idx > 0 && ends_value(&tokens[idx - 1].kind) {
                    tokens[idx].kind = TokenKind::Unit(w);
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Number scanning (doc 02 §3, doc 03 §6)
// ----------------------------------------------------------------------

fn scan_number(
    chars: &[char],
    start: usize,
    start_byte: usize,
    byte_at: &dyn Fn(usize) -> usize,
    parser_base: u32,
) -> Result<(Token, usize), Error> {
    let mut i = start;

    // ---- DMS literal: d°[m[′][s[″]]] (doc 02 §3.7) ---------------------
    if let Some((lit, next)) = try_scan_dms(chars, i, start_byte, byte_at)? {
        return Ok((
            Token::new(TokenKind::Number(lit), start_byte, byte_at(next)),
            next,
        ));
    }

    // ---- prefix (0b/0o/0x, case-insensitive) ---------------------------
    if chars[i] == '0' && is_prefix_start(chars, i) {
        let prefix_char = chars[i + 1].to_ascii_lowercase();
        let base = match prefix_char {
            'b' => 2,
            'o' => 8,
            _ => 16,
        };
        i += 2;
        let digit_start = i;
        while i < chars.len() {
            match hex_digit_value(chars[i]) {
                Some(v) if v < base => i += 1,
                Some(v) if v >= base => {
                    // Digit above the prefixed base → MP error (LIT-T004).
                    return Err(Error {
                        code: ErrorCode::Mp,
                        message: "Malformed number".to_string(),
                        token_start: Some(start_byte),
                        token_end: Some(byte_at(i + 1)),
                    });
                }
                _ => break,
            }
        }
        if i == digit_start {
            // `0x` followed by a non-digit: word fallback (0xH → variable).
            return scan_word_fallback(chars, start, start_byte, byte_at);
        }
        let int_part: String = chars[digit_start..i].iter().collect();
        let mut frac_part = String::new();
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            let fs = i;
            while i < chars.len() {
                match hex_digit_value(chars[i]) {
                    Some(v) if v < base => i += 1,
                    _ => break,
                }
            }
            frac_part = chars[fs..i].iter().collect();
        }
        // Optional base suffix; must agree with the prefix (LIT-T005/006).
        let mut base = base;
        if i < chars.len() && sub_digit(chars[i]).is_some() {
            let (run, next) = scan_sub_run(chars, i);
            let suffix = sub_value(&run).ok_or_else(|| Error::invalid(messages::MALFORMED))?;
            if suffix != base {
                return Err(Error {
                    code: ErrorCode::Invalid,
                    message: messages::MALFORMED.to_string(),
                    token_start: Some(start_byte),
                    token_end: Some(byte_at(next)),
                });
            }
            i = next;
        }
        let _ = &mut base;
        return Ok((
            Token::new(
                TokenKind::Number(Literal::Digits {
                    int_part,
                    frac_part,
                    base,
                }),
                start_byte,
                byte_at(i),
            ),
            i,
        ));
    }

    // ---- plain literal in the parser base ------------------------------
    // Digits are collected with hex-letter extension (base 11–16), then
    // validated against the *effective* base: a base suffix overrides the
    // parser base, so `2₁₀` is valid even in binary (LIT-T007).
    let mut int_digits = String::new();
    while i < chars.len() {
        match hex_digit_value(chars[i]) {
            Some(v) if v < parser_base => {
                int_digits.push(char::from_digit(v, 16).unwrap());
                i += 1;
            }
            Some(_) => {
                // Digit above the parser base: valid only when a base suffix
                // claims the whole run (LIT-T007); otherwise the literal
                // stops and the char re-scans separately.
                if let Some(ov) = try_suffix_override(chars, i) {
                    let (cont_int, cont_frac, sfx, next) = ov;
                    int_digits.push_str(&cont_int);
                    let literal = Literal::Digits {
                        int_part: int_digits,
                        frac_part: cont_frac,
                        base: sfx,
                    };
                    return Ok((
                        Token::new(TokenKind::Number(literal), start_byte, byte_at(next)),
                        next,
                    ));
                }
                break;
            }
            None => break,
        }
    }
    if int_digits.is_empty() && chars.get(i) != Some(&'.') {
        return Err(Error {
            code: ErrorCode::Invalid,
            message: messages::MALFORMED.to_string(),
            token_start: Some(start_byte),
            token_end: Some(byte_at(i)),
        });
    }
    let mut frac_digits = String::new();
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() {
            match hex_digit_value(chars[i]) {
                Some(v) if v < parser_base => {
                    frac_digits.push(char::from_digit(v, 16).unwrap());
                    i += 1;
                }
                _ => break,
            }
        }
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        // A lone radix point is not a number (EXPR-T038).
        return Err(Error {
            code: ErrorCode::Invalid,
            message: messages::MALFORMED.to_string(),
            token_start: Some(start_byte),
            token_end: Some(byte_at(i)),
        });
    }

    // Vulgar fraction directly after the integer part (must be last,
    // doc 02 §3.5).
    if i < chars.len() && frac_digits.is_empty() {
        if let Some((n, d)) = fraction_char(chars[i]) {
            let whole: u64 = if int_digits.is_empty() {
                0
            } else {
                int_digits.parse().unwrap_or(u64::MAX)
            };
            return Ok((
                Token::new(
                    TokenKind::Number(Literal::WholeAndFraction {
                        whole,
                        numerator: n,
                        denominator: d,
                    }),
                    start_byte,
                    byte_at(i + 1),
                ),
                i + 1,
            ));
        }
    }

    // Base suffix (doc 02 §3.2): overrides the parser base.
    let mut base = parser_base;
    if i < chars.len() && sub_digit(chars[i]).is_some() {
        let (run, next) = scan_sub_run(chars, i);
        let suffix = sub_value(&run).ok_or_else(|| Error::invalid(messages::MALFORMED))?;
        if !(2..=16).contains(&suffix) {
            return Err(Error {
                code: ErrorCode::Invalid,
                message: messages::MALFORMED.to_string(),
                token_start: Some(start_byte),
                token_end: Some(byte_at(next)),
            });
        }
        // All digits (including the previously accepted ones) must be valid
        // in the suffix base (LIT-T002: 2₂ errors).
        let all_text = format!("{int_digits}{frac_digits}");
        for ch in all_text.chars() {
            let v = hex_digit_value(ch).unwrap_or(99);
            if v >= suffix {
                return Err(Error {
                    code: ErrorCode::Invalid,
                    message: messages::MALFORMED.to_string(),
                    token_start: Some(start_byte),
                    token_end: Some(byte_at(next)),
                });
            }
        }
        base = suffix;
        i = next;
    }

    Ok((
        Token::new(
            TokenKind::Number(Literal::Digits {
                int_part: int_digits,
                frac_part: frac_digits,
                base,
            }),
            start_byte,
            byte_at(i),
        ),
        i,
    ))
}

/// From a digit position above the parser base, scan `digits[.digits]` plus a
/// required base suffix; returns the pieces when the suffix covers every
/// digit (LIT-T007: `2₁₀` is a decimal literal even in binary base).
fn try_suffix_override(chars: &[char], start: usize) -> Option<(String, String, u32, usize)> {
    let mut i = start;
    let mut int_part = String::new();
    while i < chars.len() {
        match hex_digit_value(chars[i]) {
            Some(v) => {
                int_part.push(char::from_digit(v, 16).unwrap());
                i += 1;
            }
            None => break,
        }
    }
    let mut frac_part = String::new();
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() {
            match hex_digit_value(chars[i]) {
                Some(v) => {
                    frac_part.push(char::from_digit(v, 16).unwrap());
                    i += 1;
                }
                None => break,
            }
        }
    }
    if i >= chars.len() || sub_digit(chars[i]).is_none() {
        return None;
    }
    let (run, next) = scan_sub_run(chars, i);
    let sfx = sub_value(&run)?;
    if !(2..=16).contains(&sfx) {
        return None;
    }
    for ch in int_part.chars().chain(frac_part.chars()) {
        if hex_digit_value(ch).is_none_or(|v| v >= sfx) {
            return None;
        }
    }
    Some((int_part, frac_part, sfx, next))
}

/// `0x`-style prefix followed by a letter: the whole alphanumeric run becomes
/// a word (variable reference, doc 02 §3.9).
fn scan_word_fallback(
    chars: &[char],
    start: usize,
    start_byte: usize,
    byte_at: &dyn Fn(usize) -> usize,
) -> Result<(Token, usize), Error> {
    let mut i = start;
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    let text: String = chars[start..i].iter().collect();
    Ok((Token::new(TokenKind::Word(text), start_byte, byte_at(i)), i))
}

/// Sexagesimal scan: integer digits then `°` starts the literal.
fn try_scan_dms(
    chars: &[char],
    start: usize,
    start_byte: usize,
    byte_at: &dyn Fn(usize) -> usize,
) -> Result<Option<(Literal, usize)>, Error> {
    let mut i = start;
    let ds = i;
    while i < chars.len() && nd_digit_value(chars[i]).is_some() {
        i += 1;
    }
    if i == ds || i >= chars.len() || !DEGREE_CHARS.contains(&chars[i]) {
        return Ok(None);
    }
    let degrees: String = chars[ds..i].iter().collect();
    i += 1; // consume °

    let mut minutes: Option<String> = None;
    let mut seconds: Option<String> = None;

    // Minutes: digits [. digits] followed by a minute mark.
    let ms = i;
    while i < chars.len() && nd_digit_value(chars[i]).is_some() {
        i += 1;
    }
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        let fs = i;
        while i < chars.len() && nd_digit_value(chars[i]).is_some() {
            i += 1;
        }
        if i == fs {
            // trailing bare decimal without a following mark is an error
            // (doc 02 §3.7) — handled below when no mark follows.
        }
        if i < chars.len() && MINUTE_CHARS.contains(&chars[i]) {
            let mut m: String = chars[ms..fs - 1].iter().collect();
            m.push('.');
            m.push_str(&chars[fs..i].iter().collect::<String>());
            minutes = Some(m);
            i += 1;
        } else {
            return Err(Error {
                code: ErrorCode::Invalid,
                message: messages::MALFORMED.to_string(),
                token_start: Some(start_byte),
                token_end: Some(byte_at(i)),
            });
        }
    } else if i > ms && i < chars.len() && MINUTE_CHARS.contains(&chars[i]) {
        minutes = Some(chars[ms..i].iter().collect());
        i += 1;
    }

    if minutes.is_some() {
        let ss = i;
        while i < chars.len() && nd_digit_value(chars[i]).is_some() {
            i += 1;
        }
        let mut sec_frac = false;
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            let fs = i;
            while i < chars.len() && nd_digit_value(chars[i]).is_some() {
                i += 1;
            }
            sec_frac = true;
            if i < chars.len() && SECOND_CHARS.contains(&chars[i]) {
                let mut s: String = chars[ss..fs - 1].iter().collect();
                s.push('.');
                s.push_str(&chars[fs..i].iter().collect::<String>());
                seconds = Some(s);
                i += 1;
            } else {
                return Err(Error {
                    code: ErrorCode::Invalid,
                    message: messages::MALFORMED.to_string(),
                    token_start: Some(start_byte),
                    token_end: Some(byte_at(i)),
                });
            }
        } else if i > ss && i < chars.len() && SECOND_CHARS.contains(&chars[i]) {
            seconds = Some(chars[ss..i].iter().collect());
            i += 1;
        }
        let _ = sec_frac;
    }

    Ok(Some((
        Literal::Dms {
            degrees,
            minutes,
            seconds,
        },
        i,
    )))
}

fn scan_sub_run(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    let mut run = String::new();
    while i < chars.len() {
        if let Some(v) = sub_digit(chars[i]) {
            run.push(char::from_digit(v, 16).unwrap());
            i += 1;
        } else {
            break;
        }
    }
    (run, i)
}

fn sub_value(run: &str) -> Option<u32> {
    run.parse::<u32>().ok()
}

/// Superscript run: optional ⁻ then ⁰-⁹; followed by √ → root index token.
fn scan_sup_run(
    chars: &[char],
    start: usize,
    start_byte: usize,
    byte_at: &dyn Fn(usize) -> usize,
) -> Result<(Token, usize), Error> {
    let mut i = start;
    let mut negative = false;
    if chars[i] == SUP_MINUS {
        negative = true;
        i += 1;
    }
    let mut digits = String::new();
    while i < chars.len() {
        if let Some(v) = sup_digit(chars[i]) {
            digits.push(char::from_digit(v, 16).unwrap());
            i += 1;
        } else {
            break;
        }
    }
    // ⁿ√ (superscript index before √) → root token (doc 02 §5.2).
    if i < chars.len() && chars[i] == '√' && !negative && !digits.is_empty() {
        let n: u32 = digits.parse().unwrap_or(0);
        return Ok((
            Token::new(TokenKind::Root(n), start_byte, byte_at(i + 1)),
            i + 1,
        ));
    }
    if negative && digits.is_empty() {
        // `2⁻` — MP error "Missing digits after minus sign" (doc 02 §5.2).
        return Err(Error {
            code: ErrorCode::Mp,
            message: messages::MISSING_MINUS_DIGITS.to_string(),
            token_start: Some(start_byte),
            token_end: Some(byte_at(i)),
        });
    }
    Ok((
        Token::new(
            TokenKind::Sup(SupPower { negative, digits }),
            start_byte,
            byte_at(i),
        ),
        i,
    ))
}
