# 09 — Compatibility Test Catalog

**Purpose:** a prioritized, independently executable test catalog for Radix, derived from the upstream test suites and the specification documents. Each test lists: ID, requirement (`10`), feature, setup, input → expected, and evidence/confidence. **Tolerance convention:** all expected strings are exact display strings at accuracy 9, FIXED or AUTOMATIC format as noted, locale C (radix `.`, separator `,`), unless stated. "Error" expectations name the ErrorCode and, where shown, the user-facing message.
**Sources:** the upstream unit test suites (≈380 equation assertions, numeric property loops, ≈50 serializer assertions) and Dogtail GUI feature files, plus spec-derived cases marked [SPEC]. Exact source-file traceability lives only in docs 01, 10, and 11.

Priorities: **P1** = core compatibility (wrong result breaks user trust), **P2** = important edge behavior, **P3** = polish/robustness.

---

## 1. Expression parsing (REQ-EXPR)

| ID | Pri | Input | Expected | Note |
|---|---|---|---|---|
| EXPR-T001 | P1 | `1+2×3` | `7` | precedence [V] |
| EXPR-T002 | P1 | `(1+2)×3` | `9` | [V] |
| EXPR-T003 | P1 | `4÷2(1+1)` | `4` | implicit multiply = multiply precedence [V] |
| EXPR-T004 | P2 | `(2)1+1` | `3` | `)number` implicit multiply [V] |
| EXPR-T005 | P2 | `(2)(1)` | `2` | [V] |
| EXPR-T006 | P1 | `−10^2` | `−100` | unary minus < power [V] |
| EXPR-T007 | P1 | `(−10)^2` | `100` | [V] |
| EXPR-T008 | P1 | `4^3^2` | `262144` | right-assoc power [V] |
| EXPR-T009 | P1 | `10 mod 4 / 2` | `1` | mod == divide precedence, left-assoc [V] |
| EXPR-T010 | P1 | `20 / 10 mod 3` | `2` | [V] |
| EXPR-T011 | P2 | `12 / 3 √4` | `8` | root at power level [V] |
| EXPR-T012 | P2 | `√5!` | `10.95445115` | factorial > root [V] |
| EXPR-T013 | P1 | `6 + 3!` | `12` | [V] |
| EXPR-T014 | P2 | `3! ^ 3` | `216`; `3 ^ 3!` | `729` [V] |
| EXPR-T015 | P1 | `100+20%` | `120` | additive percentage sugar [V] |
| EXPR-T016 | P1 | `100−20%` | `80` | [V] |
| EXPR-T017 | P2 | `100 - 20% * 100` | `80` | sugar quirk [V] |
| EXPR-T018 | P2 | `100 + 20% * 100` | `120` | [V] |
| EXPR-T019 | P2 | `100 - 100 * 20%` | `80` | [V] |
| EXPR-T020 | P2 | `(50+50)+10%=` | `110` | trailing `=` tolerated [V] |
| EXPR-T021 | P1 | `100%` | `1`; `1%` | `0.01` [V] |
| EXPR-T022 | P2 | `(((100−20)−50%)+20)` | `60` | [V] |
| EXPR-T023 | P1 | `2²` | `4`; `2¹⁰` | `1024`; `2⁻¹` | `0.5` [V] |
| EXPR-T024 | P2 | `2⁻` | Error MP "Missing digits after minus sign" | [V] |
| EXPR-T025 | P1 | `√4` | `2`; `∛8` | `2`; `∜16` | `2` [V] |
| EXPR-T026 | P2 | `₃√8` | `2`; `₁₀√1024` | `2` [V] |
| EXPR-T027 | P2 | `2√4` | `4` | implicit multiply with root [V] |
| EXPR-T028 | P2 | `x√4` (x=2) | `4` | variable root index [V] |
| EXPR-T029 | P1 | `0^0` | Error MP "Zero raised to zero is undefined" | [V] |
| EXPR-T030 | P2 | `0^0.5` | `0` | no error [V] |
| EXPR-T031 | P1 | `1+2 - 3 * 4 / 5` | `0.6` | mixed precedence [V] |
| EXPR-T032 | P2 | `+1` | `1`; `+ 1` | `1` (unary plus before number) [V] |
| EXPR-T033 | P2 | `++1` | Error INVALID | [V] |
| EXPR-T034 | P2 | `−−1` | `1` | nested unary minus allowed [V] |
| EXPR-T035 | P2 | `(50+50)+10%` | `110`; `(50+50)-10%` | `90` [V] |
| EXPR-T036 | P2 | `2×10^3` | `2000`; `2×10^−3` | `0.002` [V] |
| EXPR-T037 | P2 | `2\cdot3` | `6` | [V] |
| EXPR-T038 | P3 | empty/`+`/`.`/`−` alone | Error INVALID | [SPEC from sign_radix list] |
| EXPR-T039 | P2 | `1+` | Error INVALID (trailing operator) | [V] |
| EXPR-T040 | P2 | `a=b=c` | Error INVALID | [V] |
| EXPR-T041 | P2 | `1 2` | Error INVALID (space-separated numbers) | [SPEC] |

## 2. Literals and bases (REQ-EXPR, REQ-NUM)

| ID | Pri | Input (base) | Expected | Note |
|---|---|---|---|---|
| LIT-T001 | P1 | `1010₂` in dec | `10`; `A₁₆` | `10`; `FF` (hex base) | `FF` [V] |
| LIT-T002 | P1 | `2₂` | Error INVALID | digit above base [V] |
| LIT-T003 | P1 | `0xA` (any base) | `10`; `0b101` | `5`; `0o17` | `15` [V] |
| LIT-T004 | P2 | `0b2` | Error MP | [V] |
| LIT-T005 | P2 | `0b1₂` | `1` | prefix+suffix agree [V] |
| LIT-T006 | P2 | `0x1₂` | Error INVALID | prefix/suffix mismatch [V] |
| LIT-T007 | P2 | `2₁₀` (binary parser base) | `10` | decimal literal in binary base [V] |
| LIT-T008 | P2 | `18364758544493064720₁₀` (hex base) | `FEDCBA9876543210` | >64-bit via suffix [V] |
| LIT-T009 | P1 | `½` | `0.5`; `1½` | `1.5` [V] |
| LIT-T010 | P2 | `0°30'` | `0.5` | DMS [V] |
| LIT-T011 | P2 | `0°0'1"` | `0.000277778` | DMS seconds [V] |
| LIT-T012 | P2 | `0°0'0.1"` | `0.000027778` | [V] |
| LIT-T013 | P2 | `١٢٣٤٥٦٧٨٩٠` | `1234567890` | Arabic-Indic digits [V] |
| LIT-T014 | P2 | `۱۲۳۴۵۶۷۸۹۰` | `1234567890` | Persian digits [V] |
| LIT-T015 | P3 | `0°` | `0` | degree-only literal [V] |
| LIT-T016 | P3 | `0.10101010101010101010` (base 2) | `0.101010101` | fractional digits in base [V] |
| LIT-T017 | P3 | `1.01` | `1.01` [V] |
| LIT-T018 | P3 | `0b10103` | parse failure (null) | trailing invalid digit [V] |

## 3. Arithmetic and numeric semantics (REQ-NUM)

| ID | Pri | Input | Expected | Note |
|---|---|---|---|---|
| NUM-T001 | P1 | `0.1+0.2` | `0.3` exactly (automatic) | exact rational [V] |
| NUM-T002 | P1 | `1÷3×3` | `1` | [V] |
| NUM-T003 | P1 | `2÷3+1÷3` | `1` | [V] |
| NUM-T004 | P2 | `44.23−8.10−11.23−6.9−18` | `0` | [V] |
| NUM-T005 | P2 | `0.3333333333333333333×3−1` (3 addends per test) | `0` | 19-digit fractions exact [V] |
| NUM-T006 | P1 | `1÷0`, `0÷0`, `0.0/0.0` | Error MP "Division by zero is undefined" | [V] |
| NUM-T007 | P1 | `2^100` | `1267650600228229401496703205376` | [V] |
| NUM-T008 | P1 | `0!` | `1`; `5!` | `120`; `69!` | exact 108-digit value (see test suite) [V] |
| NUM-T009 | P2 | `3.5!` | `11.631728397`; `(−10.9)!` | `0.000003543` | Γ path [V] |
| NUM-T010 | P2 | `(−1)!`, `(−10.0)!` | Error MP | Γ pole [V] |
| NUM-T011 | P2 | `−1!` | `−1` | precedence: −(1!) [V] |
| NUM-T012 | P1 | `−1 mod 7` | `6`; `21 mod -9` | `−6`; `-21 mod 9` | `6`; `-21 mod -9` | `−3` [V] |
| NUM-T013 | P2 | `⌊3.5⌋` | `3`; `[3.5]` | `4`; `⌈3.5⌉` | `4`; `[−3.5]` | `−4` | rounding family [V] |
| NUM-T014 | P1 | `{−3.2}` | `0.8` | fractional part (floor-based) [V] |
| NUM-T015 | P2 | `frac(−3.2)` | `−0.2` | fractional component (trunc-based) [V — verified against MPFR 4.2.2] |
| NUM-T016 | P2 | `|3−5|` | `2`; `|i|` | `1`; `|3+4i|` | `5` [V] |
| NUM-T017 | P2 | `sgn −3` | `−1`; `sgn 0` | `0` [V] |
| NUM-T018 | P2 | `−0÷32352.689` | `0` | negative zero normalized [V] |
| NUM-T019 | P2 | `ncr (9;5)` | `126`; `npr (9;5)` | `15120`; `ncr (9;10)` | Error MP [V] |
| NUM-T020 | P2 | `gcd (456;1584)` | `24`; `gcd (456;1584;40;60)` | `4`; `lcm (456;1584)` | `30096` [V] |
| NUM-T021 | P2 | `gcd (456;15.84)` | Error MP | [V] |
| NUM-T022 | P3 | `1000000000000000×0.000000000000001` | `1` | precision [V] |
| NUM-T023 | P3 | value ≥ 2^2000 in FIXED | serializer error message (no result) | [V] |
| NUM-T024 | P2 | `twos 1` (hex, 32-bit) | `FFFFFFFF`; `twos 7FFFFFFF` | `80000001` [V] |
| NUM-T025 | P2 | `not 1` (hex, 32-bit) | `FFFFFFFE`; `~7A₁₆` | `FFFFFF85` [V] |
| NUM-T026 | P2 | `1100∧1010` (bin, wl4) | `1000`; `1100⊼1010` | `111`; `1100⊽1010` | `1`; `1100⊙1010` | `1001` [V] |
| NUM-T027 | P1 | `1«2` | `4`; `1«2»1«2` | `8`; `1«35` (64) | `34359738368`; `1«63` | `9223372036854775808` [V] |
| NUM-T028 | P2 | shift of value not fitting word | Error "Overflow. Try a bigger word size" | [V] |
| NUM-T029 | P3 | `modexp` small cases vs reference (e.g. 7^11 mod 13 = 2... compute) | exact | [SPEC] |
| NUM-T030 | P3 | `rand` | in [0,1), differs between reads | [V] |

## 4. Functions and constants (REQ-FUNC)

| ID | Pri | Input | Expected | Note |
|---|---|---|---|---|
| FUNC-T001 | P1 | `sin 90` (deg) | `1` exact; `sin 180` | `0` exact [V] |
| FUNC-T002 | P1 | `cos 90` | `0` exact; `cos 180` | `−1` [V] |
| FUNC-T003 | P1 | `tan 90` | Error MP (undefined tangent) | [V] |
| FUNC-T004 | P2 | `sin 45 − 1÷√2` | `0` | [V] |
| FUNC-T005 | P2 | `sin²45` | `0.5`; `cos²45` | `0.5`; `tan²10` | `0.031091204` [V] |
| FUNC-T006 | P1 | `sin (π÷2)` (rad) | `1`; `sin 100` (grad) | `1` [V] |
| FUNC-T007 | P1 | `cos⁻¹ 0` | `90` (deg); `sin⁻¹ 1` | `90`; `sin⁻¹ (−1)` | `−90` [V] |
| FUNC-T008 | P2 | `asin 1` | `90` (alias) [V] |
| FUNC-T009 | P2 | `sin 2` (radians default in tests? — degrees set) | per mode | angle dependence [V] |
| FUNC-T010 | P2 | `atanh (1÷10) − 0.5 ln(11÷9)` | `0` [V] |
| FUNC-T011 | P2 | `cosh 10 − (e^10 + e^−10)÷2` | `0`; `sinh (−10) + sinh 10` | `0` [V] |
| FUNC-T012 | P1 | `log 10` | `1`; `log 2` | `0.301029996`; `log₁₀ 10` | `1`; `log₂ 2` | `1` [V] |
| FUNC-T013 | P1 | `ln 1` | `0`; `ln 2` | `0.693147181`; `ln e` | `1` [V] |
| FUNC-T014 | P2 | `log 0`, `ln 0` | Error MP | [V] |
| FUNC-T015 | P1 | `Sqrt (2)` | `1.414213562`; `Sqrt (-1)` | `i` [V] |
| FUNC-T016 | P2 | `e^(−0.5)` | `0.60653066` [V] |
| FUNC-T017 | P1 | `π` | `3.141592654`; `e` | `2.718281828` [V] |
| FUNC-T018 | P2 | `i` | `i`; `i×i` | `−1`; `1÷i` | `−i`; `i^2` | `−1` [V] |
| FUNC-T019 | P2 | `1+i` | `1+i`; `i−1` | `−1+i`; `2i` | `2i`; `−i` | `−i` [V] |
| FUNC-T020 | P2 | `(−1)^(1/2)` | `i`; `√−4` | `2i`; `e^iπ+1` | `0` [V] |
| FUNC-T021 | P2 | `arg (1+i)` | `45`; `arg −1` | `180`; `arg −i` | `−90`; `arg 0` | Error MP [V] |
| FUNC-T022 | P2 | `log (−10) − (1 + πi÷ln(10))` | `0`; `ln (−e) − (1 + πi)` | `0` [V] |
| FUNC-T023 | P2 | `sin(iπ÷4) − i×sinh(π÷4)` | `0`; `cos(iπ÷4) − cosh(π÷4)` | `0` [V] |
| FUNC-T024 | P3 | `abs (−1)` | `1`; `abs 1` | `1` [V] |
| FUNC-T025 | P3 | unknown function `add(8)` (undefined) | Error UNKNOWN_FUNCTION; bare `add` | UNKNOWN_VARIABLE [V] |

## 5. Variables and user functions (REQ-FUNC)

| ID | Pri | Sequence | Expected | Note |
|---|---|---|---|---|
| VAR-T001 | P1 | define x=2,y=3 (host-provided in tests); `x` | `2`; `2y` | `6`; `xy` | `6`; `yx` | `6`; `2xy` | `12` [V] |
| VAR-T002 | P1 | `y²` | `9`; `x²y` | `12`; `xy²` | `18`; `(xy)²` | `36` [V] |
| VAR-T003 | P1 | `z` (undefined) | Error UNKNOWN_VARIABLE; `z2`, `z 2` | same; `z(2)` | UNKNOWN_FUNCTION [V] |
| VAR-T004 | P2 | `x³+2x²−5` | `11`; `2(x+3y)` | `22`; `x(x+3y)` | `22`; `(x+3y)(2x-4y)` | `−88` [V] |
| VAR-T005 | P1 | `z=99` | `99`, z defined [V] |
| VAR-T006 | P2 | `pi=5` → `pi` | still 3.14159… (silently not overwritten) [V] |
| VAR-T007 | P1 | `f(x;y;z)=x+y+z` then `f(2;3;5)` | definition shows `0`; call shows `10` [V] |
| VAR-T008 | P2 | redefine `f(x;y;z)=x+y-z`; `f(2;3;5)` | `0` [V] |
| VAR-T009 | P2 | `diff(x,y)=x-y` | Error INVALID (comma args) [V] |
| VAR-T010 | P2 | `funcWithDescription(x;y;z)=x+y+z @ Description` | defines; call = `10` [V] |
| VAR-T011 | P2 | nested user functions `test(test(5;6);9)` | `20` [V] |
| VAR-T012 | P2 | wrong arg count `func(x;y;z)` bare | Error UNKNOWN_VARIABLE (name parsed as vars) / count mismatch errors for defined names | [V] |
| VAR-T013 | P2 | function name = builtin (`sin(x)=x`) | Error INVALID | [V] |
| VAR-T014 | P2 | variables survive restart (registers file round-trip) | values restored | [V] |
| VAR-T015 | P2 | `_` in new expression after solve | previous answer substituted | [V] |
| VAR-T016 | P3 | multi-char var product with unknown letter `2z` | UNKNOWN_VARIABLE | [V] |

## 6. Unit/currency conversion (REQ-FUNC)

| ID | Pri | Input | Expected | Note |
|---|---|---|---|---|
| UNIT-T001 | P1 | `1 meter in mm` / `1m in mm` | `1000` [V] |
| UNIT-T002 | P1 | `1 inch in cm` | `2.54` [V] |
| UNIT-T003 | P2 | `44.45 mm in U` | `1`; `1 U in inch` | `1.75` [V] |
| UNIT-T004 | P1 | `100degC in degF` | `212`; `0degC in degF` | `32`; `0 K in degC` | `−273.15`; `1 ˚C in ˚F` | `33.8`; `-1 c in k` | `272.15` [V] |
| UNIT-T005 | P1 | `1000 bits in bytes` | `125`; `1 kB in bits` | `8000`; `1 GB in MiB` | `953.674316406` [V] |
| UNIT-T006 | P2 | `10 kibibytes in b` | `81920`; `1000 GiB in tebibytes` | `0.9765625` [V] |
| UNIT-T007 | P2 | `1000 Joule in kJ` | `1`; `3.6 MJ in kilowatthour` | `1`; `1 kWh in Megajoules` | `3.6` [V] |
| UNIT-T008 | P2 | `1 inHg in pascal` | `3386.38`; `101325 Pascal in inHg` | `29.921331924` [V] |
| UNIT-T009 | P1 | `1 kg in pounds` | `2.204622622` [V] |
| UNIT-T010 | P2 | `100 gradians in degrees` | `90` [V] |
| UNIT-T011 | P2 | `1m² in mm²` | `1000000`; `1m³ in mm³` | `1000000000` [V] |
| UNIT-T012 | P2 | `1 minute in seconds` | `60` [V] |
| UNIT-T013 | P2 | `5 m` (no `in`) | Error INVALID [V] |
| UNIT-T013b | P2 | `5 in mm` (missing from-unit) | Error INVALID [V] |
| UNIT-T014 | P2 | `5 x in y` (bad units) | Error UNKNOWN_UNIT / UNKNOWN_CONVERSION | [V] |
| UNIT-T015 | P2 | `1 meter in kg` | Error UNKNOWN_CONVERSION (cross-category) | [V] |
| UNIT-T016 | P2 | `g` bare | `9.80665` (constant wins) [V] |
| UNIT-T017 | P2 | `m` bare | Error UNKNOWN_VARIABLE [V] |
| UNIT-T018 | P2 | `˚C` bare | Error INVALID [V] |
| UNIT-T019 | P1 | `10 in bin` | displays `1010₂`; `10 in hex` | `A₁₆`; `10 in oct` | `12₈`; `10 in dec` | `10` [V] |
| UNIT-T020 | P2 | `x in bin` (x=2) | `10₂` [V] |
| UNIT-T021 | P2 | `0b1010 in dec` | `10`; `0XA in dec` | `10` [V] |
| UNIT-T022 | P2 | `1 EUR in EUR` (rates loaded) | `1` [V] |
| UNIT-T023 | P2 | offline `1 EUR in USD` (no rates) | Error UNKNOWN_RATE "Exchange rate for “USD” is not available" | [V] |
| UNIT-T024 | P3 | base conversion of large number `1234567890123456789012345678901234567890 in bin` | exact grouped binary string (test-serializer) | [V] |

## 7. Serialization and formatting (REQ-FMT)

All with grouping on, separator `,`, radix `.`, accuracy 9 [V]:

| ID | Pri | Value (base) → format | Expected |
|---|---|---|---|
| FMT-T001 | P1 | `123456789012345678901234` (10) FIXED | `123,456,789,012,345,678,901,234` |
| FMT-T002 | P1 | `0.1234567890123456789012` FIXED | `0.123456789` |
| FMT-T003 | P2 | binary `101010101010101010101010` FIXED | `1010 1010 1010 1010 1010 1010` |
| FMT-T004 | P2 | octal `123456701234567012345670` FIXED | `1234 5670 1234 5670 1234 5670` |
| FMT-T005 | P2 | hex `123456789ABCDEF012345678` FIXED | `1234 5678 9ABC DEF0 1234 5678` |
| FMT-T006 | P1 | `12345678901234567890` AUTOMATIC | `1.23456789×10¹⁹` |
| FMT-T007 | P2 | `9999999999500` AUTOMATIC | `1×10¹³` |
| FMT-T008 | P2 | 64-bit binary AUTOMATIC | plain ≤ 64 digits; 63-digit value → `1.010101011×10¹⁰⁰⁰⁰⁰⁰` (binary exponent) |
| FMT-T009 | P2 | hex `123456789ABCEDF01` AUTOMATIC | `1.23456789B×10¹⁰` (exponent in hex superscript) |
| FMT-T010 | P1 | SCIENTIFIC: `10` | `1×10¹`; `0.1` | `1×10⁻¹`; `1234567890` | `1.23456789×10⁹` |
| FMT-T011 | P2 | SCIENTIFIC non-decimal base | fixed with grouping (no exponent) |
| FMT-T012 | P1 | ENGINEERING: `0.1` | `100×10⁻³`; `0.1234567890` | `123.456789×10⁻³`; `999999999999500` | `1×10¹⁵` |
| FMT-T013 | P1 | representation base: `10` → base 2 | `1010₂`; → 8 | `12₈`; → 16 | `A₁₆` |
| FMT-T014 | P2 | `12₈` with rep-base 2 | `1010₂` |
| FMT-T015 | P3 | trailing zeroes on: `0.10` | `0.100000000` (accuracy 9); off | `0.1` |
| FMT-T016 | P2 | complex: `1+i`, `i−1`, `−i`, `2i` | `1+i`, `−1+i`, `−i`, `2i` |
| FMT-T017 | P3 | `∞`/`−∞`/`NaN` from float bit input | as-is strings |

## 8. Evaluation lifecycle / GUI behavior (REQ-UI, REQ-STATE)

| ID | Pri | Sequence | Expected |
|---|---|---|---|
| UI-T001 | P1 | type `123456789 + 987654321`, Enter | `1111111110` (dogtail) [V] |
| UI-T002 | P1 | type `5/0`, Enter | status "Division by zero is undefined", display unchanged, no history entry [V] |
| UI-T003 | P2 | `987654321^789456 +123456789` then Enter | error displayed (precision/overflow class; message per `05` §4) [V — dogtail expects "Precision error"/"Overflow: the result couldn't be calculated" — STALE, see OQ-01] |
| UI-T004 | P1 | after result, type digit | display clears, digit starts new calc [V] |
| UI-T005 | P1 | after `5+3=`, press `=` twice | 11 then 14 (repeat last op) [V] |
| UI-T006 | P2 | Escape | display cleared, history intact [V] |
| UI-T007 | P2 | Ctrl+Escape | history cleared, display intact [V] |
| UI-T008 | P2 | mode switch Basic→Programming | keypad swaps; base selector appears; result re-rendered in base |
| UI-T009 | P2 | Ctrl+B/O/D/H in programming | base changes; hex digit buttons enable/disable; display re-renders |
| UI-T010 | P2 | click bit 0 with display 5 → 4; toggle bit again → 5 | [V] |
| UI-T011 | P2 | word size 64→8 with 255 displayed | −1 (two's complement reinterpretation) [V] |
| UI-T012 | P2 | undo (Ctrl+Z) after clear | previous text restored [V] |
| UI-T013 | P2 | Alt+Left/Right | previous/next history equation in display; forward past newest restores in-progress text [V] |
| UI-T014 | P2 | copy after solve | clipboard has result without separators [V] |
| UI-T015 | P2 | paste multi-line text | newlines → spaces [V] |
| UI-T016 | P2 | accuracy change 9→2 with 3.141592654 shown | `3.14`; value stored unchanged (next calc uses full value) [V] |
| UI-T017 | P2 | define variable via popover with invalid name `1x` | error message "Variable name can not start with a digit"; add disabled [V] |
| UI-T018 | P3 | `= ` when display is `f(x)` | inserts `=` (function header detection) [V] |
| UI-T019 | P3 | financial dialog Ctrm with pint=0.1, fv=2, pv=1 | result ≈ ln2/ln1.1 = 7.272540897 (display replaces expression, not in history) [S] |
| UI-T020 | P3 | character dialog 'A' → insert | `65` inserted [S] |

## 9. CLI / search provider / persistence (REQ-STATE)

| ID | Pri | Action | Expected |
|---|---|---|---|
| CLI-T001 | P1 | `gnome-calculator --solve "2+2"` | stdout `4\n`, exit 0 [V] |
| CLI-T002 | P2 | `--solve "1/0"` | stderr `Error: Division by zero is undefined`, exit 1 [V] |
| CLI-T003 | P2 | `--solve "1+"` | stderr `Error: ErrorCode.INVALID`, exit 1 [V] |
| CLI-T004 | P2 | `--solve "10 in hex"` | stdout `A₁₆\n` [V] |
| CLI-T005 | P2 | `--version` | `<name> <version>` on stderr, exit 0 [V] |
| CLI-T006 | P2 | `gcalccmd` REPL: `2+3` then `exit` | `> 5`, clean exit [V] |
| CLI-T007 | P3 | `gcalccmd` with EOF | newline handling, exit 0 [V] |
| SP-T001 | P1 | Shell query `2+2` | 1 result, description ` = 4` [V] |
| SP-T002 | P2 | query `5` (plain number) | no results [V] |
| SP-T003 | P1 | query `x=5` (contains =) | no results [V] |
| SP-T004 | P2 | query `987654321^789456` | no results (budget) [V] |
| SP-T005 | P2 | activate result | result in clipboard + notification [V] |
| STATE-T001 | P1 | solve `1+1`, close app, reopen | history shows `1+1 / 2` [V] |
| STATE-T002 | P1 | Clear History, close, reopen | empty history [V] |
| STATE-T003 | P2 | set mode Programming, close, reopen | Programming restored [V] |
| STATE-T004 | P2 | define variable `a=5`, restart | `a` still defined (registers file) [V] |
| STATE-T005 | P2 | define function, restart | still defined (custom-functions file) [V] |
| STATE-T006 | P3 | resize/maximize, close, reopen | size/maximized restored [V] |
| STATE-T007 | P3 | refresh-interval = 0, clear cache, restart | no network fetch attempted; offline rates stay absent [V] |

## 10. Localization cases (REQ-FMT)

| ID | Pri | Scenario | Expected |
|---|---|---|---|
| LOC-T001 | P1 | locale with radix `,` (e.g. de_DE): GUI `2,5+1` | parsed as 2.5+1 = `3,5` displayed [V] |
| LOC-T002 | P2 | same locale CLI `--solve "2,5+1"` | normalization: tsep removed, radix→`.`; result `3,5` [V] |
| LOC-T003 | P2 | locale with tsep `.` | no separator stripping; radix replacement safe [V] |
| LOC-T004 | P3 | hex result in locale with localized digit glyphs | default build shows ASCII digits [V] |
| LOC-T005 | P3 | translated `in`/`to` keywords (e.g. de `in`) | conversion works with translated keyword [S] |
| LOC-T006 | P3 | RTL locale | display/keypads stay LTR; converter fields right-justified [V] |

## 11. Coverage-gap notes [SPEC]

Behaviors with NO upstream test (define tests during implementation, mark inferred):
- Engineering/scientific format interaction with `show-zeroes`.
- Repeat-last-operation across modes and after base changes.
- Undo stack depth limits (none observed).
- Currency conversion arithmetic with real provider data (only EUR→EUR tested).
- DMS converter round-trips beyond the unit tests above.
- Very long expressions (UI performance), deep bracket nesting.
- Dogtail GUI expectations contain stale error strings (OQ-01) — GUI tests should use `05` §4 messages.

**Execution notes for Team B:** headless engine tests (sections 1–7) must run without GUI; GUI behaviors (section 8) should be automated at the accessibility/UI layer; CLI tests (section 9) via subprocess. All exact strings assume C locale unless the test says otherwise.
