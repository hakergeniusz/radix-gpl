# 03 — Numeric Semantics

**Scope:** the numeric model of the original: representation, precision, rounding, exact rational arithmetic, complex numbers, domains, and edge cases of every arithmetic operation. This is the document that makes results reproducible bit-for-bit at the observable level.
**Evidence:** full read of the original numeric core and its arbitrary-precision math-library bindings, the application's precision wiring, its numeric and equation test suites, and the settings schema. Confidence markers as in `02`.

---

## 1. Representation model [V]

A value ("Number") is a **complex number** whose real and imaginary parts are arbitrary-precision reals, plus an optional **exact rational shadow**:

- Real/imaginary parts are computed at a configurable working precision `P` (MPFR bits, **default 2000** via GSchema key `precision`; library fallback default is 1000 if unset).
- When a value is exactly representable as a rational p/q (any operation that keeps values exact maintains this), an exact GMP rational is carried alongside; arithmetic on two rationals is performed **exactly** and only re-rounded to the MPFR part for display (v51 change "Keep rational arithmetic exact"). Consequences observable in tests: `0.1+0.2` = 0.3 exactly; `1÷3×3` = 1; `2÷3+1÷3` = 1; `0.3333333333333333333+0.3333333333333333333+0.3333333333333333334−1` = 0 (19-digit fractions cancel exactly); `1−0.9−0.1` = 0.
- The rational shadow is dropped (falling back to rounded binary values) by any operation that can be inexact: roots of non-perfect powers, powers with non-rational results, transcendental functions, etc. Example artifact: `44.23−8.10−11.23−6.9−18` = 0 exactly [V, automatic format].
- There is no separate integer type: integers are rationals/complexes that happen to be integral. Values larger than 64 bits are supported natively (`2^100` = 1267650600228229401496703205376 [V]).
- Signed zero: negative zero is normalized to `+0` in results (`−0÷32352.689` = 0 [V]); `arg` of a negative real returns +π (−π from the underlying library is corrected) [V].
- Infinity/NaN: not produced by arithmetic (division by zero errors instead), but can arise from float input (bit patterns); they display as `∞`, `−∞`, `NaN` [V].

## 2. Precision and rounding [V]

- Working precision `P` = 2000 bits (GSchema `precision`, range effectively 1…any int; exposed in neither preferences dialog nor menu — it is a dconf-only setting).
- All MPFR/MPC operations round to nearest (ties-to-even) at `P` bits.
- **Display precision is independent of working precision**: display accuracy `A` (GSchema `accuracy`, default 9 digits after the radix) controls how many fractional digits are shown, but the *stored* value keeps full precision. Subsequent calculations use the stored value, not the displayed one.
- Values with more than `P` significant bits can only arise via irrational intermediate results; the observable limit for exactness is the rational shadow (exact for all rational inputs/outputs).
- **FIXED-format display cap:** results whose magnitude exceeds **2^P** (i.e. 2^2000) cannot be rendered in fixed/automatic formats: error "The result is too long to display in fixed format. Try other result formats" [V]. This cap does not apply to scientific/engineering formats nor to internal bitwise conversions.
- Display rounding (serializer): a number is rounded for display by adding base^(−(A+1))·base/2, i.e. **round half away from zero at digit A+1** — e.g. `2÷3` shows 0.666666667 at accuracy 9 [V]; `0.000277778` for 1″ in degrees [V]. Trailing zeros are stripped unless "show trailing zeroes" is on.

## 3. Complex numbers [V]

- The imaginary unit is `i`. Any operation may produce a complex result: `√−1` = i, `√−4` = 2i, `(−1)^0.5` = i, `ln(−10) = ln 10 + πi`, `e^iπ` = −1 (exact via identity handling in tests: `e^iπ+1` = 0).
- A complex result displays as `a+bi` with rules: if the imaginary part formats as 0 the result displays real; `+1i`/`−1i` display as `+i`/`−i`; a real part of 0 is omitted (no leading `+0`); ASCII letter `i` with Unicode minus signs (`1−i`, `i−1` → `−1+i`).
- `arg 0` errors ("Argument not defined for zero"); `arg` of a negative real returns +π (180°) not −π [V].
- Division by zero is an error even for complex operands (`i÷0` → "Division by zero is undefined").
- Inverting a real number yields imaginary component exactly `+0i` (v51 change; keeps `arg`/`ln` of such values consistent).

## 4. Operation semantics [V unless noted]

### 4.1 Arithmetic
- `+ − ×`: exact when both operands carry the rational shadow; otherwise MPFR at P bits.
- `÷`: division by zero (real or complex zero divisor, including `0.0/0.0`) → MP error "Division by zero is undefined". Exact rational division otherwise.
- `mod` (`x mod y`): `z = x − floor(x/y)·y`, then if y<0 and z>0, or y>0 and z<0, then z += y. Verified: `−1 mod 7` = 6; `21 mod -9` = −6; `-21 mod 9` = 6; `-21 mod -9` = −3; `0 mod 7` = 0. Complex operands error ("Modulus division is only defined for real numbers").
- `modexp` (`x^y mod p`): modular exponentiation by repeated squaring with `mod` reduction; negative exponents use the reciprocal of x; negative modulus behaves as `mod`. Budget-limited in search context only.

### 4.2 Powers and roots
- `x^y`: if y is a non-complex non-integer whose reciprocal is an integer n, computes `root(n)(x)` instead of general power (so `4^0.5` = 2; `8^(1÷3)` path exists via `(−8)^(1÷3)` = −2 [V — negative base odd root is real]). General case uses complex power (mpc).
- `0^0` → MP error "Zero raised to zero is undefined" (returns 0 with error set); `0^−n` → "The power of zero is undefined for a negative exponent". `2^0` = 1, `x^1` = x.
- Integer superscript powers (`x²`, `2¹⁰`) use dedicated integer-power code — same results, separate code path with identical limits.
- `√`, `∛`, `∜`, `ⁿ√`: `root(n)`. n=0 errors ("The zeroth root of a number is undefined"); negative n computes the reciprocal then the positive root. Negative base with odd n → real root (`₃√−8` = −2); negative base with even n → principal complex root (`√−4` = 2i).
- `sqrt` of negative → complex (above).

### 4.3 Logarithms and exponentials
- `ln 0` → "Logarithm of zero is undefined"; `ln x` for x<0 → `ln|x| + πi` [V: `ln (−e) − (1 + πi)` = 0].
- `log x` = log₁₀ x; `log(x;y)` = log_y(x) = ln x / ln y; `log_n` (subscript digit forms `log₂`…`log₉…`) same as 2-arg log. `log 0`/`ln 0` error; base 0 or 1 → "Logarithm with base 0 or 1 is undefined". `log (−10) − (1 + πi÷ln(10))` = 0 [V].
- `e^x` (natural exponential; computed over the complex field, so complex exponents work): `e^(−0.5)` = 0.60653066 [V].

### 4.4 Trigonometry [V]
- Angle interpretation from the active angle unit (degrees default; radians; gradians). Internally converted by multiplying by π/180 or π/200 (and the reverse for inverses).
- Exactness guards: `sin` of an angle that is an integer multiple of 180° (deg), π (rad), 200 (grad) returns exactly 0; `cos` of 180°·n+90° (or π/2·n, 100·n grad) returns exactly 0; `tan` of multiples of 180° returns 0 and of 90°+180°·n errors "Tangent is undefined for angles that are multiples of π (180°) from π∕2 (90°)". The radian case uses an epsilon of |x|·2^(−P) for the near-multiple test. This is why `sin 90` = 1 and `sin 180` = 0 come out exact in degrees.
- Inverse trig (`asin`, `acos`, `tan⁻¹`): real results outside domain error ("Inverse sine is undefined for values outside [-1, 1]" etc.); results converted to the active angle unit; `atan(i)`/`atan(−i)` error. `cos⁻¹ 0` = 90 (deg), `sin⁻¹ (−1)` = −90.
- Hyperbolic functions (sinh, cosh, tanh + inverses) have no angle dependence. `acosh x` for real x<1 errors; `atanh x` for real |x|≥1 errors (condition as coded: x≥1 or x≤−1) with "Inverse hyperbolic tangent is undefined for values outside [-1, 1]".

### 4.5 Factorial [V]
- `0!` = 1; positive integers exact (`69!` printed in full [V]); negative *integers* → "Factorial is undefined for negative integers"; negative non-integers and positive non-integers use Γ(x+1): `3.5!` = 11.631728397, `(−10.9)!` = 0.000003543, `(−10.0)!` errors (Γ(−9) pole), `−1!` = −1 (unary minus applied to `1!` — parses as `−(1!)`), `(−1)!` errors. Complex factorial errors ("Factorial of complex numbers is not supported").
- Combinatorics: `npr(n;r)` = n·(n−1)···(n−r+1) (iterative; `npr(9;0)` = 1); `ncr(n;r)` = npr(min(r, n−r))/r!-style symmetric computation. Both require 0 ≤ r ≤ n (naturals), else MP errors. Verified: `ncr(9;5)` = 126, `npr(9;5)` = 15120; `ncr(9;10)`, `ncr(9;−5)` error.

### 4.6 Rounding family [V]
- `[x]` / `round`: ties away from zero (MPFR round): `[3.5]` = 4, `[−3.5]` = −4.
- `⌊x⌋` floor toward −∞, `⌈x⌉` ceiling toward +∞: `⌊−3.2⌋` = −4, `⌈−3.2⌉` = −3.
- Two distinct fractional functions exist:
  - `{x}` (**fractional part**, parser brackets) = x − floor(x), always in [0,1): `{3.2}` = 0.2, `{−3.2}` = 0.8 [V].
  - `frac x` (**fractional component**, function) = x − trunc(x), sign follows x: `frac(−3.2)` = −0.2 [V — verified against MPFR 4.2.2 `mpfr_frac` behavior directly].
  - `int x` integer component = trunc toward 0.
- floor/ceil/round of complex numbers error ("Floor is only defined for real numbers" etc.).

### 4.7 Other functions with numeric edge cases [V]
- `abs` of complex = modulus (|3+4i| = 5). `sgn 0` = 0; `sgn` of complex = x/|x|.
- `cmp(x;y)` Kronecker delta: 1 if equal else 0.
- `gcd`/`lcm` accept 1..n integer arguments (1 arg = |x|; 0 and error on non-integers). `gcd(456;1584)` = 24; `gcd(456;1584;40;60)` = 4; `lcm(456;1584)` = 30096. `gcd(456;15.84)` errors.
- Statistics: `sum`, `sumsq`, `average`, `median` (mean of two middle values for even n), `min`/`max` (real only; complex errors), `var`/`varp` = Σ|x−mean|²/(n−1 or n) (absolute value squared — works for complex too), `stdev`/`stdevp` = square roots of those; `stdev`/`var` require ≥ 2 arguments.
- `twos(x)` two's complement within word size (`twos 1` = FFFFFFFF at 32-bit); `bswap(x)` byte-swap within word size.
- `factorize`: prime factorization with multiplicity, ascending; sign applied to the first factor for negative input; 0 → [0]; 1 → [1]; primality for large values via deterministic Miller–Rabin variant (SPRP bases 2,3,5,7,11,13,17,19,23,29,31,37,41 for ln(x)+1 < 56, else the Sorenson–Webster extended base set) — probabilistic only in theory, deterministic for 64-bit.

## 5. Word size and integer behavior [V]

- Word size (8/16/32/64) applies to: bitwise NOT/NAND/NOR/XNOR, shifts, `twos`, `bswap`, the programming-mode bit panel, and overflow checks for shifts.
- When the word size changes, the currently displayed number is *reinterpreted* in two's complement of the new size (e.g. −1 at 64-bit stays −1 at 8-bit → displays −1; 255 at 64-bit → −1 at 8-bit) [V]. Floats and complex numbers are left alone.
- There is no integer overflow error for plain arithmetic (rationals are unbounded); the only overflow errors are: shifts/bitwise-adjacent operations exceeding the word ("Overflow. Try a bigger word size") and MPFR overflow of exp/powers ("Overflow error" / "Underflow error" via MPFR flags).
- The programming-mode help states: "Attempting to enter a larger number than will fit in the selected word size will result in an Overflow error" — this refers to shifts/bitwise paths; plain entry of larger numbers is allowed and just displays.

## 6. Parsing numbers to values [V]

The literal parser accepts: optional `0b/0o/0x` prefix (must agree with base suffix), optional sign (parsed by the grammar, not here), digits in the base (Unicode Nd digits allowed), optional `.` fraction, optional vulgar-fraction char, optional base suffix. Anything unconsumed → failure (the candidate is rejected and the scanner falls back to word tokenization). Sexagesimal `d°m′s″` handled before base parsing (requires `°`).

## 7. Random [V]

`rand` produces a fresh uniform value in [0,1) **on each read** (GLib double RNG, seeded from wall-clock microseconds at process start). It is a variable read, so `rand` in an expression references one value per read position.

## 8. Comparison and equality [V]

- No relational operators exist in the language. `cmp(x;y)` (Kronecker delta) is the only equality test. Internal comparisons (unit eligibility, overflow checks) use exact MPFR comparison of real parts.
- Display equality: values are compared for display decisions via string forms (e.g. zero_string detection), which is observable in edge formatting cases (e.g. complex numbers with tiny imaginary parts display as real when the formatted imaginary part rounds to zero string).

## 9. Boundary behavior summary

| Case | Behavior |
|---|---|
| Largest displayable fixed value | < 2^2000 (else format error) |
| `1÷3×3` | exactly 1 (rational shadow) |
| `0.1+0.2` | exactly 0.3 |
| `2^100` | exact integer |
| `69!` | exact integer (Γ path, exact because rational? no — Γ(70) at 2000 bits is exact for integers up to 2^2000; displayed in full) |
| `tan 90` (degrees) | MP error (documented message) |
| `arg 0` | MP error |
| `0^0`, `0^−1` | MP errors (values 0 displayed with error) |
| `(−1)!`, `(−10.0)!` | MP error (Γ pole) |
| `gcd` with non-integers | MP error |
| shift exceeding word | MP error "Overflow. Try a bigger word size" |
| result ≥ 2^2000 in FIXED/AUTOMATIC | serializer error message, no result shown |

## 10. Known surprising-but-real behaviors (must reproduce)

1. `4÷2(1+1)` = 4, not 1: implicit multiplication has multiply precedence, evaluated left-to-right with division [V].
2. `{−3.2}` = 0.8 (floor-based fractional part) while `frac(−3.2)` = −0.2 (truncation-based) [V].
3. Percentage sugar quirks: `100 - 20% * 100` = 80; `100 + 20% * 100` = 120; `(50+50)+10%=` = 110 [V].
4. `1000000000000000÷1000000000000000` = 1 and the 19-digit fraction cancellation = 0 (exact rational shadow) [V].
5. `−1!` = −1 but `(−1)!` errors (precedence: factorial binds tighter than unary minus) [V].
6. `0.10101010101010101010` in binary displays `0.101010101` (9 fractional digits at accuracy 9) — fractional digits counted in the display base [V].
7. `sin 90` returns exactly 1 in degrees due to the 180°·n guard, not floating-point rounding [V].
