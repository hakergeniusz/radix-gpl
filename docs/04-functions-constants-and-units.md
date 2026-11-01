# 04 — Functions, Constants, and Units Catalog

**Scope:** every built-in function, constant, unit, currency, and financial computation, with aliases, signatures, exact semantics, domains, error cases, and examples. Numeric foundations are in `03`; syntax in `02`.
**Evidence:** direct source analysis of the function registry, financial formulas, unit definitions and conversion formulas, currency handling, the constants catalog, the financial keypad, and help pages; cross-checked against tests. Confidence: **V** verified, **S** strongly supported, **I** inferred.

---

## 1. Built-in functions [V]

All functions are looked up case-insensitively (exact match first, then lowercase). Argument separator is `;` (comma-separated argument lists are a definition-syntax error). Invocations may omit brackets for a single argument (`sin 30`); `name⁻¹` powers dispatch to the matching inverse (see `02` §5.2). Argument-count mismatches are `MP` errors: "Function “%s” takes N argument(s)"; `log` accepts 1 or 2; `stdev`/`var` require ≥ 2; variadic functions (`…` last parameter) accept ≥ 1.

### 1.1 Trigonometry (angle-unit aware: radians/degrees/gradians, see `03` §4.4)

| Name | Aliases | Args | Definition | Domain | Errors |
|---|---|---|---|---|---|
| `sin` | — | 1 | sin of angle | all (complex supported) | — |
| `cos` | — | 1 | cos of angle | all | — |
| `tan` | — | 1 | tan of angle | all | angle ≡ 90° (mod 180°) → "Tangent is undefined…" |
| `sin⁻¹` | `asin` | 1 | inverse sine, result in active angle unit | real [−1,1]; complex | outside → "Inverse sine is undefined for values outside [-1, 1]" |
| `cos⁻¹` | `acos` | 1 | inverse cosine | real [−1,1]; complex | outside → analogous message |
| `tan⁻¹` | `atan` | 1 | inverse tangent | all real; complex | x = ±i → "Inverse tangent is undefined for values i and -i" |
| `sinh` | — | 1 | hyperbolic sine | all | — |
| `cosh` | — | 1 | hyperbolic cosine | all | — |
| `tanh` | — | 1 | hyperbolic tangent | all | — |
| `sinh⁻¹` | `asinh` | 1 | inverse hyperbolic sine | all | — |
| `cosh⁻¹` | `acosh` | 1 | inverse hyperbolic cosine | real ≥ 1; complex | real < 1 → "Inverse hyperbolic cosine is undefined for values less than one" |
| `tanh⁻¹` | `atanh` | 1 | inverse hyperbolic tangent | real (−1,1); complex | real |x| ≥ 1 → "Inverse hyperbolic tangent is undefined for values outside [-1, 1]" |

Note: aliases `asin`/`acos`/`atan`/`asinh`/`acosh`/`atanh` are registered as separate function entries mapping to the same operations. `arg` is listed under Complex below but is also angle-unit aware (result expressed in active angle unit).

### 1.2 Complex

| Name | Args | Definition | Errors |
|---|---|---|---|
| `conj` | 1 | complex conjugate | — |
| `arg` | 1 | complex argument (phase), in active angle unit | x = 0 → "Argument not defined for zero" |
| `re` | 1 | real component | — |
| `im` | 1 | imaginary component | — |

### 1.3 Programming (word-size dependent, `wordlen`)

| Name | Args | Definition | Errors |
|---|---|---|---|
| `twos` | 1 | two's complement within word size: `not(x) + 1` | non-natural → "Two’s complement is only defined for non-negative integers" |
| `bswap` | 1 | byte-swap within word size (reverse wordlen/8 bytes; sign-extends negatives) | non-integer → "Swap endianness is only possible on integers"; overflow → "Overflow. Try a bigger word size" |
| `mod` | 2 | modulus divide (`03` §4.1) | complex → error |
| `modexp` | 3 | modular exponentiation x^y mod p | budget-limited only |

### 1.4 Rounding

| Name | Args | Definition | Errors |
|---|---|---|---|
| `round` | 1 | nearest integer, ties away from zero | complex → "Round is only defined for real numbers" |
| `floor` | 1 | floor | complex → "Floor is only defined for real numbers" |
| `ceil` | 1 | ceiling | complex → "Ceiling is only defined for real numbers" |
| `int` | 1 | integer component (truncate toward 0) | — |
| `frac` | 1 | fractional component x − trunc(x) (sign of x) — **differs from `{ }`** (`03` §4.6) | — |

### 1.5 Statistics

| Name | Args | Definition | Errors |
|---|---|---|---|
| `sum` | ≥1 (variadic) | Σxᵢ | — |
| `sumsq` | ≥1 | Σxᵢ² | — |
| `average` | ≥1 | Σxᵢ/n | — |
| `median` | ≥1 | middle value (sorted); mean of two middles for even n | complex → "Median is only defined for real numbers" |
| `min` / `max` | ≥1 | minimum/maximum | complex → "Minimum value is only defined for real numbers" / "Maximum value…" |
| `stdev` | ≥2 | √(sample variance) | 1 arg → "Function “%s” takes at least 2 arguments" |
| `stdevp` | ≥1 | √(population variance) | — |
| `var` | ≥2 | sample variance: Σ(xᵢ−mean)²/(n−1) | 1 arg → "…takes at least 2 arguments" |
| `varp` | ≥1 | population variance: Σ(xᵢ−mean)²/n | — |

(The squared deviation is computed as |xᵢ−mean|², so complex inputs do not error for variance/stdev.)

### 1.6 Sundry

| Name | Args | Definition | Errors |
|---|---|---|---|
| `log` | 1–2 | log₁₀(x), or log_y(x) with second arg; also `log_n` subscript forms (`log₂`…`log₉` + multi-digit) | x=0 → "Logarithm of zero is undefined"; base 0 or 1 → "Logarithm with base 0 or 1 is undefined"; >2 args → "Function “%s” takes 1 or 2 arguments" |
| `ln` | 1 | natural logarithm; ln(x<0) = ln|x| + πi | x=0 error |
| `sqrt` | 1 | square root (complex for negatives) | — |
| `abs` | 1 | absolute value / complex modulus | — |
| `sgn` | 1 | signum: −1/0/1; complex → x/\|x\| | — |
| `ncr` | 2 | combinations C(n,r) | non-natural or n<r → MP errors ("Combination is only defined for non-negative integers" / "Combination is undefined if n is less than r") |
| `npr` | 2 | permutations P(n,r) | analogous errors |
| `gcd` | ≥1 variadic | greatest common divisor (Euclid via mod) | non-integers → "Greatest common divisor is only defined for integers" |
| `lcm` | ≥1 variadic | least common multiple: \|x/gcd·y\| | non-integers → "Least common multiple is only defined for integers" |
| `cmp` | 2 | Kronecker delta: 1 if x=y else 0 | — |

**Dynamic function names [V]:** `log` immediately followed by subscript digits (e.g. `log₂`, `log₁₆`) is a defined function name for any base ≥ 0 expressed in subscripts, equivalent to `log(x; base)` with the base appended as the last argument. The parser also supports `log 2 (8)`-style: `log` + bare number base + bracketed argument.

### 1.7 User-defined functions [V]

- Definition syntax: `name(params) = expression [@ description]` typed directly into the display and committed with Enter (the definition statement evaluates to 0).
- Name rules: must not collide with built-in function or operator names, reserved variables, constants, or existing variables; must not start with a digit; only letters/digits/underscores. Violations produce status messages: "Can not use defined variable or constant names", "Can not use built-in function or operator names", "Function name can not start with a digit", "Function name can only contain digits, letters and underscores" (validation in the popover) and `INVALID` at parse time. The definition requires purely alphabetic parameter names.
- Persistence: `$XDG_DATA_HOME/gnome-calculator/custom-functions`, one definition per line, format `name(p1;p2)=expression@description`.
- Evaluation: body re-parsed with parameters bound by position; parameters shadow globals; unresolved names fall back to global variables/constants. Function bodies can use units/conversions and call other user functions. Wrong argument count errors at call time.
- Management UI: f(x) popover — insert, edit (loads `name(args)=expr@description` into the display), delete (custom only). Categories shown for built-ins: Trigonometry, Complex, Programming, Rounding, Statistics, Sundry.

## 2. Constants [V]

Constants are variables that exist always and cannot be reassigned (assignment silently ignored). Recognized names: `pi`, `tau`, `i` (lowercase ASCII only) plus the symbols below (exact Unicode). Displayed in the variable popover grouped by category:

| Category | Symbol | Name | Exact stored value |
|---|---|---|---|
| Mathematics | `e` | Euler’s Number | MPFR e (computed as exp(1) at precision P) |
| Mathematics | `π` | Pi | MPFR π at precision P |
| Mathematics | `τ` | Tau | MPFR 2π at precision P |
| Mathematics | `φ` | Golden Ratio | (1+√5)/2 computed at precision P |
| Electromagnetism | `c₀` | Light Speed | 299792458 (integer) |
| Electromagnetism | `е` | Elementary Charge | 0.0000000000000000001602176634 |
| Electromagnetism | `Z₀` | Impedance of Vacuum | 376.730313412 |
| Electromagnetism | `μ₀` | Permeability of Vacuum | 0.00000125663706127 |
| Electromagnetism | `ε₀` | Permittivity of Vacuum | 0.0000000000088541878188 |
| Atomic & Nuclear | `h` | Planck’s Constant | 0.000000000000000000000000000000000662607015 |
| Atomic & Nuclear | `α` | Fine-Structure Constant | 0.0072973525643 |
| Atomic & Nuclear | `mₑ` | Electron Mass | 0.00000000000000000000000000000091093837139 |
| Atomic & Nuclear | `mₚ` | Proton Mass | 0.00000000000000000000000000167262192595 |
| Thermodynamics | `k` | Boltzmann Constant | 0.00000000000000000000001380649 |
| Thermodynamics | `mᵤ` | Atomic Mass Unit | 0.00000000000000000000000000166053906892 |
| Thermodynamics | `R` | Molar Gas Constant | 8.31446261815324 |
| Thermodynamics | `σ` | Stefan-Boltzmann Constant | 0.00000005670374419 |
| Thermodynamics | `Nᴀ` | Avogadro’s Number | 602214076000000000000000 |
| Gravitation | `G` | Gravitational Constant | 0.000000000066743 |
| Gravitation | `g` | Earth Acceleration | 9.80665 |

Notes: the elementary-charge symbol is the **Cyrillic** letter `е` (U+0435), not Latin `e` — a long-standing quirk; the Latin `e` is Euler's number. Decimal strings are parsed at full precision via the exact-rational path. Test values: `π` displays 3.141592654, `e` displays 2.718281828 (accuracy 9).

**Pseudo-variables [V]:** `_` = result of previous calculation; `rand` = fresh uniform random [0,1) per read. Neither can be deleted via the popover (reserved).

## 3. Units and conversion [V]

Conversion syntax: `x unit1 in unit2` (the from-unit is mandatory), `x in bin/oct/dec/hex` for base representation. Conversion chains through a category base: value converted from source unit to base then to target (temperature uses offset arithmetic, not pure factors). Exact conversion expressions (variable `x`) per unit are below; "→" means to-base, "←" from-base.

### 3.1 Categories and units (display order; names and all accepted symbols)

**Angle** (base: radian)
| Unit | Symbols | → base | ← base |
|---|---|---|---|
| Degrees (Decimal) `degree` | degree, degrees, deg | πx/180 | 180x/π |
| Degrees (D°M′S″) `dms` | (none — special UI unit) | πx/180 | 180x/π |
| Radians `radian` | radian, radians, rad | x | x |
| Gradians `gradian` | gradian, gradians, grad | πx/200 | 200x/π |

**Length** (base: meter): parsec (pc) ×30857000000000000; lightyear (ly) ×9460730472580800; astronomical-unit (au) ×149597870700; rack-unit (U) ÷22.49718785151856; nautical-mile (nmi) ×1852; mile (mile, miles, mi) ×1609.344; kilometer (km, kms) ×1000; cable (cb) ×219.456; fathom (ftm) ×1.8288; meter (m) ×1; yard (yd) ×0.9144; foot (ft) ×0.3048; inch (in) ×0.0254; centimeter (cm, cms) ÷100; millimeter (mm) ÷1000; micrometer (um) ÷1000000; nanometer (nm) ÷1000000000; point (point, pt, points, pts) ×0.000352777778.

**Area** (base: square-meter): hectare (ha) ×10000; acre ×4046.8564224; square-mile (mi²) ×2589988.110336; square-foot (ft²) ÷10.763910417; square-kilometer (km²) ×1000000; square-meter (m²) ×1; square-centimeter (cm²) ×0.0001; square-millimeter (mm²) ×0.000001.

**Volume** (base: litre): cubic-meter (m³) ×1000; imperial-gallon (imp_gallon, imp_gallons, imp_gal) ×4.54609; gallon (US, gal) ×3.785412; litre (L, liter(s), litre(s)) ×1; quart (qt) ×0.9463529; imperial-pint (imp_pt…) ×0.56826125; pint (pt) ×0.4731765; cup (cp) ×0.25; millilitre (mL, cm³) ×0.001; microlitre (mm³, μL, uL) ×0.000001.

**Mass** (display name "Mass", category key "weight"; base: kilograms): tonne ×1000; kilograms (kg, kgs) ×1; pound (lb, lbs) ×0.45359237; ounce (oz) ×0.02834952; troy-ounce (ozt) ×0.0311034768; gram (g) ×0.001; milligram (mg) ×0.000001; microgram (μg, ug) ×0.000000001; stone (st) ×6.350293.

**Duration** (base: second): century ×3155760000; decade ×315576000; year ×31557600 (Julian year); month ×2629800; week ×604800; day ×86400; hour ×3600; minute ×60; second (s) ×1; millisecond (ms) ×0.001; microsecond (us, μs) ×0.000001; nanosecond (ns) ×0.000000001.

**Speed** (base: meters-second): kilometers-hour (kmph, kmh, kph) ÷3.6; miles-hour (miph, mph, mi/h…) ÷2.23693629; meters-second (mps) ×1; feet-second (fps…) ÷3.28084; knot (kt, kn, nd) ÷1.94384449.

**Frequency** (base: hertz): hertz (Hz) ×1; kilohertz (kHz) ×1000; megahertz (MHz) ×1000000; gigahertz (GHz) ×10⁹; terahertz (THz) ×10¹².

**Temperature** (base: kelvin; OFFSET conversions): degree-celsius (degC, °C, ˚C, C, c, Celsius, celsius): → x+273.15, ← x−273.15; degree-fahrenheit (degF, °F, ˚F, F, f, Fahrenheit, fahrenheit): → (x+459.67)×5/9, ← x×9/5−459.67; degree-kelvin (k, K, Kelvin, kelvin): identity; degree-rankine (degR, °R, ˚R, °Ra, ˚Ra, r, R, Rankine, rankine): → x×5/9, ← x×9/5.

**Energy** (base: joule): joule (J, j…) ×1; kilojoule (KJ…) ×1000; megajoule (MJ) ×10⁶; kilowatthour (kwh, kWh) ×3600000; btu (BTU) ×1054.350264489; calorie (cal) ×4.184; kilocalorie (kcal) ×4184; erg ÷10000000; ev (electronvolt(s)) ×1.602176634/10¹⁹... (source: x*1.602176634/10000000000000000000, i.e. ×1.602176634e-19); ftlb (foot-pound(s), ft-lb, ft-lbs) ×1.3558179483314004.

**Pressure** (base: pascal): pascal (Pa) ×1; kilopascal (kPa) ×1000; hectopascal (hPa) ×100; bar ×100000; atm ×101325; mmhg (mmHg) ×133.322387415; inhg (inHg) ×3386.38; psi ×6894.759.

**Digital Storage** (base: byte; SI for kB, IEC for KiB): bit (b, bits) ÷8; byte (B) ×1; nibble ÷2; kilobit (kb, Kb) ×1000/8; kilobyte (kB, KB) ×1000; kibibit (Kib) ×1024/8; kibibyte (KiB) ×1024; megabit (Mb) ×10⁶/8; megabyte (MB) ×10⁶; mebibit (Mib) ×2²⁰/8; mebibyte (MiB) ×2²⁰; gigabit (Gb) ×10⁹/8; gigabyte (GB) ×10⁹; gibibit (Gib) ×2³⁰/8; gibibyte (GiB) ×2³⁰; terabit (Tb) ×10¹²/8; terabyte (TB) ×10¹²; tebibit (Tib) ×2⁴⁰/8; tebibyte (TiB) ×2⁴⁰; petabit (Pb) ×10¹⁵/8; petabyte (PB) ×10¹⁵; pebibit (Pib) ×2⁵⁰/8; pebibyte (PiB) ×2⁵⁰; exabit (Eb) ×10¹⁸/8; exabyte (EB) ×10¹⁸; exbibit (Eib) ×2⁶⁰/8; exbibyte (EiB) ×2⁶⁰; zettabit (Zb) ×10²¹/8; zettabyte (ZB) ×10²¹; zebibit (Zib) ×2⁷⁰/8; zebibyte (ZiB) ×2⁷⁰; yottabit (Yb) ×10²⁴/8; yottabyte (YB) ×10²⁴; yobibit (Yib) ×2⁸⁰/8; yobibyte (YiB) ×2⁸⁰.

**Currency** (dynamic; base: none — rate-based, §3.2).

**Number Base** (special converter category; units named "2","8","10","16" — Binary, Octal, Decimal, Hexadecimal; identity conversion; used by the converter UI only).

Ambiguity rule [V]: if a symbol matches more than one unit (e.g. `pt` matches both Desktop Publishing Points and US Pints; `r` and `R` match Rankine and Molar Gas... `R` is a constant — constants take precedence as variables; unit matching `R` matches Rankine), and more than one match exists **within the same lookup pass** (case-sensitive, then case-insensitive across all categories), the symbol resolves to *no unit at all* (`get_unit_by_symbol` returns null when count > 1). Verified conversion examples: `44.45 mm in U` = 1; `1 U in inch` = 1.75; `1 GB in MiB` = 953.674316406; `1 inHg in pascal` = 3386.38; `101325 Pascal in inHg` = 29.921331924; `100degC in degF` = 212; `0 K in degC` = −273.15.

Note `g` resolves to the *constant* 9.80665 (not a unit) — `g` as a bare token is a variable [V]; `m` alone is `UNKNOWN_VARIABLE` (not meter) [V].

### 3.2 Currency [V]

- 67 built-in currencies (name, display name, symbol), e.g. USD "US Dollar" "$", EUR "Euro" "€", JPY "Japanese Yen" "¥", INR "Indian Rupee" "₹", ISK, LKR, KGS, UZS, VND, XAF, XOF, ZAR, etc. (full list recorded in traceability doc 01; providers can add more).
- Values are **rates fetched online**; a currency without a rate cannot be used (conversion → `UNKNOWN_RATE` error "Exchange rate for “%s” is not available").
- Conversion arithmetic: `x from→to = x / value(from) × value(to)`.
- Providers (in registration/attempt order): IMF (rms_five.xls; rates are reciprocals of raw file values; 36 name-mapped currencies), UN Treasury (un-daily.xls; value = USD-value × raw; CRLF/TSV format, fields: [4]=rate, [3]=date, [2]=symbol), ECB (eurofxref-daily.xml; value = EUR-value × raw rate; plus fixed rates RSD=EUR/0.0085, XAF=XOF=EUR/0.00152449), Bank of Canada (fxtwdcad.xml for TWD; value = CAD-value / raw). First provider to set a currency wins; later providers fill only currencies that still lack rates.
- Rate files cached in `$XDG_CACHE_HOME/gnome-calculator/` (eurofxref-daily.xml, rms_five.xls, un-daily.xls, fxtwdcad.xml); downloads go through `https://exchange-api.gnome.org/...`.
- Refresh: GSchema `refresh-interval` seconds (UI choices: Never=0, Daily=86400, Weekly=604800; default Weekly). 0 = never fetch (offline uses stale cache if present). Files older than the interval (or empty) are re-fetched at startup and when the setting changes. Failed downloads log a warning and leave rates as-is.
- Converter shows an "Exchange Rates Not Loaded" pane with a "Refresh rates" link until rates load; a Rate Details dialog shows each side's provider name and rate date (locale date format) with "Not Available" fallback, plus an attribution link to the provider.

## 4. Financial functions [V]

All financial functions are **dialog-only** (buttons in financial mode; "Financial functions cannot be performed using the keyboard" per help). Each dialog has numeric entries (defaults "0"; non-numeric or sign-only content resets to 0 on focus loss; entries accept only numbers); pressing the last entry's Enter or the Calculate button runs the computation and **replaces the display with the result** (not added to history).

| Button | Dialog title | Fields (in order) | Formula |
|---|---|---|---|
| Ctrm | Compounding Term | Periodic Interest Rate (pint), Future Value (fv), Present Value (pv) | ln(fv/pv) / ln(1+pint) |
| Ddb | Double-Declining Depreciation | Cost, Life, Period | iterate `VAL = (cost − bv)·2/life`, `bv += VAL` for `period` periods (period truncated to integer; negative period shows status "Error: the number of periods must be positive" but still computes with 0 iterations) |
| Fv | Future Value | Periodic Payment (pmt), Periodic Interest Rate (pint), Number of Periods (n) | pmt·((1+pint)ⁿ − 1)/pint |
| Gpm | Gross Profit Margin | Cost, Margin | cost/(1−margin) |
| Pmt | Periodic Payment | Principal, Periodic Interest Rate, Term | prin·pint/(1−(1+pint)^(−n)) |
| Pv | Present Value | Periodic Payment, Periodic Interest Rate, Number of Periods | pmt·(1−(1+pint)^(−n))/pint |
| Rate | Periodic Interest Rate | Future Value, Present Value, Term | (fv/pv)^(1/n) − 1 |
| Sln | Straight-Line Depreciation | Cost, Salvage, Life | (cost−salvage)/life |
| Syd | Sum-of-the-Years’-Digits Depreciation | Cost, Salvage, Life, Period | (cost−salvage)·(life−period+1)/((life·(life+1))/2) |
| Term | Payment Period | Periodic Payment, Future Value, Periodic Interest Rate | ln(1 + fv·pint/pmt)/ln(1+pint) |

Sign conventions: payments and values are entered as raw positive/negative numbers; no annuity-type switches exist. Interest rate `pint` is per period as a fraction (0.05 = 5%).

Domain notes: pint=0 → division by zero error for Fv/Pmt/Pv; fv/pv ≤ 0 → ln domain error for Ctrm; Term with fv·pint/pmt ≤ −1 → ln domain error. Errors surface through the standard status line.

Also in financial mode: a **Currency Conversion** button that switches to conversion view with the Currency category preselected.

## 5. Mode availability summary [V]

- Scientific functions/complex numbers/constants: any mode (language is uniform); the *buttons* differ per mode.
- Unit/currency conversion: via `in` syntax in all modes (including programming), or via the dedicated conversion mode (see `06` §6). The converter panel is not embedded in basic/advanced/financial keypads.
- Programming functions (`twos`, `bswap`, bitwise ops, shifts): language-wide; word size from settings (default 64).
- Financial dialogs: financial mode only.
