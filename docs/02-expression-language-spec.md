# 02 — Expression Language Specification

**Scope:** the expression language accepted by the calculator: literals, identifiers, operators, precedence, conversions, assignments, function definitions, whitespace, locale-sensitive input, and malformed-input behavior. Numeric semantics of the values are in `03`; function/constant/unit catalogs in `04`; error surfacing in `05`.
**Evidence:** all claims are derived from direct analysis of the original engine source and cross-checked against its unit test suite and help pages. Confidence is marked per section: **V** = verified by clear code path and/or test, **S** = strongly supported, **I** = inferred. Exact source-file traceability lives only in docs 01, 10, and 11.

---

## 1. Terminology

- **Canonical form:** the expression text after the UI has normalized it (see §11). The parser operates on canonical form.
- **Parser number base** (`base`): the base in which bare literals are interpreted when no explicit base is given. 2–16; selected in programming mode, 10 otherwise.
- **Representation base:** the base in which the *result* is displayed (set by `in bin/oct/dec/hex` conversions, see §8).
- **Word size** (`wordlen`): 8/16/32/64 bits, affecting bitwise/shift semantics (§7.6).

## 2. Character set and tokens [V]

Input is UTF-8 text. The pre-lexer classifies individual characters:

| Character(s) | Token |
|---|---|
| `.` and `,` | decimal point (either accepted anywhere as radix) |
| Unicode decimal digits `0-9` and **any Unicode Nd digit** (e.g. Arabic-Indic `١٢٣`, Devanagari) | digit |
| `a-f`, `A-F` | hex digit (when part of a number) or letter (when starting a word) |
| `⁰¹²³⁴⁵⁶⁷⁸⁹` | superscript digit; `⁻` superscript minus |
| `₀-₉` | subscript digit |
| `½⅓⅔¼¾⅕⅖⅗⅘⅙⅚⅛⅜⅝⅞` | fraction characters |
| `°`, `˚` | degree sign; `'`, `′`, `’` minute sign; `"`, `″`, `”` second sign |
| letters (any Unicode alphabetic), `_`, `\` | letter (words) |
| `+` | add |
| `-`, `−`, `–` | subtract (ASCII hyphen, minus sign, en dash) |
| `*`, `×` | multiply |
| `/`, `∕`, `÷` | divide |
| `(` `)` `[` `]` `{` `}` `|` | grouping (parentheses / round / fractional-component / absolute-value) |
| `^` | power |
| `!` | factorial |
| `%` | percentage |
| `;` | function-argument separator |
| `@` | function-description separator |
| `=` | assignment |
| `⌊` `⌋` `⌈` `⌉` | floor / ceiling brackets |
| `√` `∛` `∜` | square / cube / 4th root |
| `∧` `⊼` `∨` `⊽` `⊻` `⊕` `⊙` `¬` `~` | bitwise AND/NAND/OR/NOR/XOR/XOR/XNOR/NOT/NOT |
| `«` `≪` | shift left; `»` `≫` shift right; `⋙` unsigned shift right |
| space, tab, newline, CR | whitespace (skipped) |
| anything else | unknown → `INVALID` error |

Whitespace is allowed anywhere between tokens and is ignored.

**Digit classification detail [V]:** digit detection uses Unicode's Nd (decimal number) category; letter detection uses Unicode alphabetic property. Hex-digit value of non-ASCII decimal digits is their Unicode numeric value (verified against GLib `g_unichar_isxdigit`/`g_unichar_xdigit_value` semantics, which the number parser relies on). The test suite parses `١٢٣٤٥٦٧٨٩٠` (Arabic-Indic) and `۱۲۳۴۵۶۷۸۹۰` (Persian) as `1234567890`.

## 3. Numbers and literals [V]

A **number token** is recognized by the scanner's digit/hex/decimal scan plus a full-validity check (the candidate must consume entirely as a literal of the relevant base — see §6). The general form:

```
[number] = [prefix] [sign]? digits [radix digits] [base-suffix] | [prefix] digits | fraction-atom ...
```

1. **Plain literals** are interpreted in the *parser number base*. In base 10, digits with an optional fractional part (`1.5`, `.5`, `1.`). In bases 2/8/16, digits valid for the base (binary `1010₂`-style literals also work when typed in a non-matching base if they are valid there — see base-suffix rules below).
2. **Base suffix [V]:** any literal may carry a subscript base suffix `₂`…`₁₆` (composed of subscript digits, e.g. `1010₂`, `FF₁₆`). The suffix overrides the parser base. Suffix must be 2–16; `2₁₀` (number `2`, base 10) parses because 2 ≤ 9 ≤ digits allowed in base 10. `2₂` is an error (`INVALID`), `A₂` errors as `UNKNOWN_VARIABLE` (no digits precede the suffix, so it is not a number; `A` becomes a variable token).
3. **Literal prefixes [V]:** `0b`/`0B` (binary), `0o`/`0O` (octal), `0x`/`0X` (hex) — case-insensitive prefix letter. `0x1F` works in any parser base. A prefix and a base suffix must agree (`0b1₂` valid, `0x1₂` is `INVALID`). Malformed prefixed literals produce `MP` errors (`0b2` → error "…"), while a stray `0x` followed by a non-hex word becomes a variable reference (`0xH` → `UNKNOWN_VARIABLE`).
4. **Signs:** `+`/`−`/`-` before a literal are handled by the grammar (unary plus/minus), not by the literal.
5. **Fractions [V]:** the 15 Unicode vulgar-fraction characters are accepted as literals, optionally after an integer (`½` = 0.5, `1½` = 1.5). A fraction must be the last thing in the literal.
6. **Scientific-notation entry [V]:** there is no `1e5` syntax; scientific literals are written as products (`2×10^3`, `2×10⁻³`). The keypad's exponent button inserts `×10` with superscript mode (§06).
7. **Sexagesimal (DMS) literals [V]:** `d°[m[′|'|’][s[″|"|”]]]` forms are parsed as `d + m/60 + s/3600`. The degree sign is mandatory; minutes/seconds may be decimal (`0°30.5′` is accepted; a trailing bare decimal without a following minute mark is an error). `0°` and `1°` are valid. `0°30'` = 0.5; `0°0'1"` = 0.000277778 (rounded at 9 places in the test).
8. **Localized radix [V]:** the display's canonical form replaces the locale radix with `.` before parsing, so the parser only ever sees `.` (and `,` is treated as a radix character by the pre-lexer but never survives canonicalization in the GUI paths; the CLI and search provider perform the same replacement).
9. **Thousands separators [V]:** removed from the canonical expression before evaluation; they never reach the parser in GUI/CLI flows.

**Invalid number examples** (all produce errors rather than silent misreads): `1..2`, `0b2`, `0b1₂` mismatch `0x1₂`, digits above the base (`2₂`), empty after prefix (`0b` followed by non-digit → variable reference `UNKNOWN_VARIABLE`).

## 4. Identifiers: variables, functions, units, currencies [V]

A **word** is a run of letter characters (any Unicode alphabetic), `_`, `\`, optionally terminated by **subscript digits** (which are part of the word). Word classification, in priority order:

1. **Operator words** (checked case-sensitively on the lowercased word): `mod`, `and`, `nand`, `or`, `nor`, `xor`, `xnor`, `nxor`, `not`, `\cdot` (multiply). The conversion keywords `in` and `to` are matched **against the current translation** (i.e. localized).
2. **Function** if a function of that name is defined (built-in, `log` + subscript digits like `log₂`, or user-defined; case-insensitive lookup: exact first, then lowercased).
3. **Currency** if a currency has that name and it has a rate.
4. **Unit** if a unit symbol matches (units may be suffixed by superscript digits, e.g. `m²`).
5. **Variable** otherwise.

After lexing, any `UNIT`/`CURRENCY` token that is not adjacent to an `IN` keyword (and does not start with `°`) is reclassified as a `VARIABLE`. So `m` alone is a variable reference, not a unit.

- **Multi-character variables [V]:** if an undefined word is not a defined variable, it is interpreted as the *product of its single-character variables*: `xy` = `x·y` (both must be defined; otherwise `UNKNOWN_VARIABLE`). Superscripts after a word apply to the last character only (`xy²` = `x·y²`).
- **Case sensitivity [V]:** variables are case-sensitive when defined by the host (`variables.get(name)`); constants `pi`/`tau`/`i` are lowercase-only (`Pi` is not the constant). Function lookup is case-insensitive. Unit symbol matching is case-sensitive first, then case-insensitive (ambiguities — a symbol matching two units — resolve to *no unit*).
- **Reserved names [V]:** `_` (previous answer) and `rand` (random [0,1)) are always-defined variables. The constants `pi`, `tau`, `i` plus all physical-constant symbols (see `04` §2) behave as variables that cannot be reassigned (assignment to them is silently ignored; the expression still evaluates).

## 5. Operators, precedence, associativity [V]

Precedence (higher binds tighter) and associativity from the parser (verified against the precedence table in source and the test suite):

| Prec | Operator(s) | Arity | Assoc | Notes |
|---|---|---|---|---|
| 1 | `x in y` (conversion) | binary | — | lowest; see §8 |
| 2 | (grouping depth barrier) | — | — | parentheses create a depth level that bounds operator reach |
| 3 | `+` `−` | binary | left | with percentage sugar (§5.1) |
| 4 | implicit/`×` `*` `\cdot` | binary | left | |
| 5 | `mod` | binary | left | same level as divide |
| 5 | `/` `÷` `∕` | binary | left | |
| 6 | `not` `¬` `~` | unary | right | operand to the right; word-size masked |
| 7 | function application | unary | right | `sin 30`, `sin(30)` |
| 8 | `and` `nand` `or` `nor` `xor` `xnor` | binary | left | bitwise |
| 8 | `≪` `»` `≫` `⋙` (`<<` `>>` `>>>`) | binary | left | shifts |
| 9 | `%` postfix | unary | right | divides by 100 |
| 10 | unary `−` | unary | right | |
| 10 | `^` | binary | right | `4^3^2` = 4^(3^2) |
| 10 | `√` `∛` `∜` `ⁿ√` `ₙ√` | unary | right | root binds like power |
| 11 | `!` postfix | unary | right | factorial |
| 12 | numbers, variables, superscript powers `x²` `x⁻¹` | | | highest |

Representative verified examples (from the upstream equation test suite):

- `1+2×3` = 7; `(1+2)×3` = 9; `1 - 0.9 - 0.1` = 0
- `2(1+1)` = 4 (implicit multiply after `)`) [V]; `(2)1+1` = 3; `4÷2(1+1)` = 4 [V — implicit multiply binds at multiply level, so this is (4÷2)×(1+1)]
- `−10^2` = −100 (unary minus binds looser than power); `(−10)^2` = 100
- `10 mod 4 / 2` = 1 (left-to-right at equal precedence); `20 / 10 mod 3` = 2
- `12 / 3 √4` = 8; `√5!` = √(5!) = √120 ≈ 10.95445115 (factorial binds tighter than root)
- `4 ^ sin 30` = 2; `sin 30 ^ 4` = sin(30⁴); `sin (30 ^ 4)` = sin(810000) = 0 (degrees)
- `6 + 3!` = 12; `100 mod 3!` = 4; `3 ^ 3!` = 729; `3! ^ 3` = 216
- `1«2` = 4; `1«2»1«2` = 8 (shifts left-assoc); `4^3^2` = 262144 (right-assoc power)
- `10 / - 2` = −5; `sin - 30` = −0.5 (unary minus after function keyword is its argument's sign)
- `x³+2x²−5` with x=2 → 11; `2(x+3y)` = 22; `x(x+3y)` = 22; `(x+3y)(2x-4y)` = −88

### 5.1 Percentage semantics [V — core cases; S — complex cases]

- Postfix `%`: `x%` = x/100. `100%` = 1; `1%` = 0.01; `100×1%` = 1; `100÷1%` = 10000.
- **Additive percentage sugar:** in `a + b%` and `a − b%` where `b` is a simple term, the operation becomes `a·(1 ± b/100)`: `100+1%` = 101, `100−1%` = 99, `140+15%` = 161 (documented in help), `(((100−20)−50%)+20)` = 60.
- With a compound right side the sugar applies to the whole right subtree with quirky results: `100 - 20% * 100` = 80, `100 + 20% * 100` = 120, `100 - 100 * 20%` = 80, `(50+50)+10%` = 110 [all asserted in tests]. Implementers must reproduce these exactly; the upstream source marks this logic with a FIXME, so treat the tests as the normative behavior.
- A trailing `=` after a percentage expression is tolerated and ignored: `(50+50)+10%=` = 110 [V].

### 5.2 Superscript/subscript power and roots [V]

- `x²`, `x³`, `x⁻¹`, `2¹⁰` are integer powers with the same precedence as `^` (`x⁻¹` where x=2 → 0.5; `2⁻¹` = 0.5). `2⁻` (no digits after superscript minus) is an `MP` error.
- Roots: `√x` (2nd), `∛x` (3rd), `∜x` (4th), `ⁿ√x`/`ₙ√x` (nth via superscript/subscript digits). `₃√8` = 2; `₁₀√1024` = 2; `√(2+2)` = 2; `2√4` = 2·√4 = 4 (implicit multiply of number and root). Zeroth root: error "The zeroth root of a number is undefined".
- Function powers: `sin²45` = (sin 45)² = 0.5; `sin⁻¹x` dispatches to the inverse function (see `04` §1); `log₂ 2` = 1.

## 6. Grouping [V]

- `( )`: plain grouping; nesting supported; depth resets operator precedence so conversions can't escape (see §8).
- `[ ]`: **round** to nearest integer (`[3.7]` = 4, `[−3.7]` = −4 — ties away from zero). Not a list/interval syntax.
- `{ }`: **fractional part** = x − floor(x), always in [0,1) (`{3.2}` = 0.2, `{−3.2}` = 0.8; note this differs from the `frac()` function, which is truncation-based — see `03` §4.6).
- `⌊ ⌋` floor, `⌈ ⌉` ceiling, `| |` absolute value: unary operators over a nested expression (`|3−5|` = 2).
- **Smart bracket insertion is a display feature** (see `06`): typing `(` inserts a pair or the closing bracket contextually; the parser still requires balanced brackets. Unclosed brackets at evaluation time: the parser fails (`INVALID`); the UI additionally extends error highlighting to cover auto-insertable closing brackets (observable behavior: error selection expands over missing `)`s).

## 7. Bitwise operators [V]

- `and`/`∧`, `nand`/`⊼`, `or`/`∨`, `nor`/`⊽`, `xor`/`⊻`/`⊕`, `xnor`/`⊙`/`nxor`, `not`/`¬`/`~`.
- Operands must be **non-negative integers** (`is_natural`), else `MP` error ("Bitwise … is only defined for non-negative integers").
- Semantics: operands converted to hex digit strings; the operation applies **per hex digit (nibble)**:
  - `and`, `or`, `xor`: result width = max(operand widths) — **no word-size masking**.
  - `not`, `nand`, `nor`, `xnor`: result width = word size (wordlen/4 hex digits) — masked to the word size.
- Examples (base 16, wordlen 32): `not 1` = `FFFFFFFE`; `not 7FFFFFFF` = `80000000`; `twos 1` = `FFFFFFFF`; `~7A₁₆` = `FFFFFF85`. Base 2, wordlen 4: `1100∧1010` = `1000`; `1100⊼1010` = `111`; `1100⊽1010` = `1`; `1100⊙1010` = `1001`.

### 7.6 Shifts [V]

- `≪`/`<<` (left), `≫`/`>>` (right), `⋙`/`>>>` (unsigned right). Shift count is reduced modulo the word size. Both operands must be integers and fit the word size (signed for negative operands), else `MP` error "Overflow. Try a bigger word size" / "Shift is only possible on integers".
- Left shift of a negative value sign-extends within the word size. Right shift is arithmetic (sign-preserving) for negative operands, logical for non-negative. Unsigned right shift masks to the word size first, then shifts logically, producing a non-negative result.
- `1«35` (wordlen 64) = 34359738368 (35 ≡ 35 mod 64 — no reduction needed); `1«63` = 9223372036854775808 [V].

## 8. Conversions [V]

Syntax (all lowercase keywords `in`/`to` are matched against the current locale's translation):

| Form | Meaning |
|---|---|
| `x unit1 in unit2` | physical-unit conversion: the **from-unit is mandatory** (`1m in mm`, `100 gradians in degrees`). `x in unit2` without a from-unit is `INVALID`. |
| `x in bin` / `oct` / `dec` / `hex` (or `binary`, `octal`, `decimal`, `hexadecimal`) | **representation-base conversion**: re-render the result in that base (only recognized when immediately followed by end-of-expression) |
| `x cur1 in cur2` | currency conversion (currency names recognized only when adjacent to `in`; from-currency also mandatory, e.g. `1 EUR in USD`) |

- Verified examples: `10 in bin` → displayed `1010₂`; `10 in hex` → `A₁₆`; `1010₂ in dec` → `10`; `0b1010 in dec` → `10`; `1 meter in mm` = 1000; `100degC in degF` = 212; `100 gradians in degrees` = 90.
- Unit-conversion result value: the *numeric* value after conversion; the unit is not part of the value. Base-conversion results carry the subscript base suffix when the representation base differs from the parser base.
- Errors: unknown unit → `UNKNOWN_UNIT` ("Unknown unit “%s”"); units from different categories → `UNKNOWN_CONVERSION` ("Unknown conversion"); currency without a loaded rate → `UNKNOWN_RATE` ("Exchange rate for “%s” is not available"); unknown base keyword → `UNKNOWN_CONVERSION`.
- Unit names are recognized **without** `in` only when directly followed by `in` (`5m in ft`); a trailing bare unit (`5 m`) is `INVALID`, and a missing from-unit (`5 in mm`) is `INVALID` as well.

## 9. Assignment and function definition [V]

- `name = expression`: evaluates the right side, **stores it as variable `name`**, and the statement's value is the right side (so `z=99` displays 99). Names of built-in constants (π, e, τ, i, physical constants) are silently *not* overwritten. Only one `=` is allowed per expression (`a=b=c` is `INVALID`).
- `name(param1;param2;…) = expression [@ description]`: defines a user function. Name and parameters must be purely alphabetic (Unicode letters; no digits/underscores — enforced at definition). Parameters are separated by `;` (comma-separated parameter lists are `INVALID`). The body is stored, not evaluated; the definition evaluates to 0. `diff(x,y)=x-y` → `INVALID` [V]. Redefining a built-in function name fails (`INVALID`); redefining a custom function replaces it. The body may reference other user functions (recursion via self-reference is possible but has no guard other than evaluation limits).
- Function invocation: `name(arg1;arg2;…)` or `name arg` (single argument without brackets). Arguments are full expressions; `;` separates them only at the top nesting depth. Argument-count mismatch produces `MP` errors ("Function “%s” takes N argument(s)").

## 10. Expression-level error classes [V]

| ErrorCode | Meaning | Typical trigger |
|---|---|---|
| `NONE` | success | |
| `INVALID` | syntax error | unknown character, trailing tokens, `a=b=c`, unbalanced brackets |
| `UNKNOWN_VARIABLE` | variable not defined | `z` (undefined), `A` outside hex |
| `UNKNOWN_FUNCTION` | function not defined | `z(2)` where z is a variable |
| `UNKNOWN_CONVERSION` | bad conversion | incompatible unit categories, bad base keyword |
| `UNKNOWN_UNIT` | unit symbol unknown | `5 x in y` |
| `UNKNOWN_RATE` | currency rate missing | offline currency conversion |
| `MP` | mathematical/parse-level error | division by zero, domain errors, malformed literals, message-carrying errors |
| `TIMED_OUT` | evaluation deadline exceeded | search-provider budget only |
| `RESOURCE_LIMIT` | input/token/output/step/exponent limit exceeded | search-provider budget only |

Full user-facing message strings: `05` §4.

## 11. Canonicalization between display and parser [V]

The GUI display shows formatted text (locale radix, thousands separators, Unicode minus `−`, `×10` superscript exponents, bold answer region). Before evaluation, the display text is canonicalized:

1. The answer region (previous result shown bold) is replaced by the placeholder `_ ` (underscore + space).
2. Thousands separators are removed (only when between digits).
3. The locale radix is replaced by `.`.
4. `\\cdot` sequences are literal LaTeX-style multiply (`2\\cdot3` = 6 [V]).

The reverse happens when results are shown (`05` §3). This canonicalization is why the parser itself only ever handles `.` and no separators.

## 12. Implicit multiplication summary [V]

Implicit multiplication is inserted between: number and variable/function/root/subscript (`2y`, `2sin` — note `2sin` after a digit is multiply + function), two grouped expressions (`(2)(1)`), closing bracket and number (`(2)1+1`), variable and variable/function/root (`xy`, `x√4`), number and fraction char (`1½`). It is NOT inserted between a number and a unit without `in` (that is an error), nor between number and `(` when the number is the whole left side of an `in` conversion (`2(1+1)` = 4 shows it *is* inserted there — the number-`(` case is handled).

## 13. Partial expressions during editing [V]

During typing, evaluation errors are expected and non-fatal: incomplete input ("1+", "(1", "sin ") shows either a transient result of the parseable prefix or an error status (see `05`). Examples: after typing `1+` the parser reports `INVALID` (trailing operator) and the previous display state is kept; the status line stays empty until a committed solve (live-evaluation errors are not displayed while typing — only the last committed state and current error status are shown; verified from display logic: errors set the status text only via `solve`).
