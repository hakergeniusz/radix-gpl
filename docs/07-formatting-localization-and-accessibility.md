# 07 — Formatting, Localization, and Accessibility

**Scope:** how values become display text and how display text is interpreted: separators, formats, bases, sign handling, precision, locale behavior, translated strings, and accessibility-relevant behavior. Numeric semantics are in `03`.
**Evidence:** full read of the original output-formatting component and the display reformat logic, the CLI and search-provider entry points, the settings schema, the upstream serialization test suite, and the translation catalog (102 languages). Confidence: **V** / **S** / **I**.

---

## 1. Locale data used [V]

From the C locale (`nl_langinfo`): the radix character (decimal separator) and thousands separator, taken at serializer construction. If the locale's thousands separator is `.` (used as radix in some locales), it is not used for grouping. Grouping count is 3 in decimal and **4 in non-decimal bases**. The serializer exposes a secondary "narrow" space separator used for grouping in non-decimal bases (the group character for bases ≠ 10 is a space-like character, U+202F narrow no-break space in the source); `is_thousands_separator` accepts both the locale separator and that space.

## 2. Display formats [V]

`number-format` setting (GSchema enum): automatic (default), fixed, scientific, engineering.

### 2.1 Fixed [V]
- Round to `accuracy` (A) fractional digits **half away from zero** (adds base^−(A+1)·base/2 before truncating digits).
- Integer part: digits grouped every `tsep_count` (3 or 4) when grouping enabled.
- Fraction: up to A digits; stops early when the remaining fraction is zero; trailing zeros stripped unless show-zeroes and A > 0.
- Radix character from locale; minus sign is U+2212 `−`.
- Numbers with |x| ≥ 2^precision (default 2^2000) cannot be rendered: error "The result is too long to display in fixed format. Try other result formats".
- Base suffix: when the representation base differs from the parser base, a subscript base suffix is appended (`1010₂`, `A₁₆` — multi-digit suffixes like `₁₆` use two subscript digits).

### 2.2 Automatic [V]
- Fixed rendering first; if the total digit count ≤ threshold (and the value isn't zero) show it, else scientific. Thresholds by representation base: 2 → 64 digits, 8 → 22, 16 → 16, 10 → leading-digits (default 12; set to the accuracy value at runtime? — verified: default 12; the GUI does not change it; CLI/search use defaults).
- Zero shows as `0`.

### 2.3 Scientific [V]
- Base 10 only: mantissa in [1,10) with A fractional digits, then `×10` with the exponent in **superscript digits** (`1.23456789×10⁹`). Negative exponents use superscript minus `⁻` (`1×10⁻¹`). No exponent shown for 10⁰ (`1` displays plain).
- In non-decimal bases scientific format falls back to fixed (with grouping).
- Verified: `10` → `1×10¹`; `1234567890` → `1.23456789×10⁹`; `0.1` → `1×10⁻¹`; `9999999999500` → `1×10¹³` (rounding cascades into the exponent).

### 2.4 Engineering [V]
- Like scientific but the exponent is rounded down to a multiple of 3 (`0.1` → `100×10⁻³`; `0.1234567890` → `123.456789×10⁻³`; `999999999999500` → `1×10¹⁵`). Mantissa overflow fixups: if rounding pushes the mantissa to `10…` it is renormalized (exponent+1), and to `1000…` (exponent+3) in engineering.

### 2.5 Non-decimal base display [V]
- In programming mode with base 2/8/16, results display in that base with grouping every 4 digits (narrow space), e.g. binary `1010 1010 1010 …`, hex `1234 5678 9ABC DEF0`.
- The exponent of automatic/scientific overflow in non-decimal bases is rendered in the **current base** with superscript digits including `ᴬ ᴮ ᶜ ᴰ ᴱ ᶠ` for 10–15 (v51 "Show exponent in the current base"): hex `123456789ABCEDF01` → `1.23456789B×10¹⁰` (exponent 10₁₆ = 16 decimal — wait, verified example: exponent superscript "10" in hex = 16 decimal; the test asserts exactly `1.23456789B×10¹⁰`).
- Base-suffix subscripts appear when a base-conversion equation changes the representation base (fixed format), e.g. `1010₂`, `12₈`, `A₁₆`, and a 200-bit binary number grouped in 4s with a trailing `₂`.

### 2.6 Complex formatting [V]
- `a+bi` with real part omitted when zero (`i`, `2i`); imaginary `1` shown as bare `i` with forced sign (`1+i`, `1−i`, `−1+i`); `i⁻¹` → `−i`. Unicode minus; ASCII `i`. Zero imaginary part → real number display. Zero real part with tiny imaginary rounding to zero → `0`.

### 2.7 DMS formatting [V]
- The converter's DMS unit displays values as `d°m′s″` (e.g. angles), with carry handling (60″ → +1′, 60′ → +1°) and an epsilon (2^(8−P)) guard snapping near-integers.

## 3. Sign and zero handling [V]

- Minus is always U+2212 `−` in output; ASCII `-` accepted on input.
- Negative zero normalized away (`−0÷x` → `0`).
- Trailing-zero suppression by default; `show-zeroes` pads to the accuracy digit count (zero string computed per current settings).
- The "zero string" (formatted zero) is used to decide answer-region reformatting and complex real-part suppression.

## 4. Display-to-engine canonicalization [V]

(Reference — detailed in `02` §11.) Before evaluation: answer region → `_ `; separators removed; locale radix → `.`. The CLI/search/pipe inputs: thousands separator removed (unless it is `.`), locale radix replaced by `.`. The search provider additionally **rejects plain numbers** (double-parseable) and any input containing `=`.

## 5. Reformatting on setting changes [V]

Accuracy/grouping/trailing-zero/format changes re-render: the answer region (from the stored value), history answer labels (re-serialized from stored values; restored-from-tape entries are NOT re-rendered — they keep their stored strings), and the converter fields. Base changes re-render the display number as a plain literal (fixed format, no trailing zeros) if the current text is a plain number; a committed result re-renders with base suffix rules.

## 6. Localization [V]

- All UI strings are gettext-translated (102 languages). Function names/aliases are NOT translated (help explicitly says do not translate `sin`), but the conversion keywords `in`/`to` ARE matched against translations, as is the constants' display names, unit names/symbols are not translated (symbols fixed), unit category names and currency display names are translated.
- Display digits: the digit glyphs for hex results (`get_digit_text`) are taken from the translation of "0,1,2,3,4,5,6,7,8,9,A,B,C,D,E,F" if a translation provides localized digit characters; default build uses ASCII digits (the localized-digit feature is compiled to default-off: `use_default_digits = true`).
- Input accepts any Unicode decimal digits for values 0-9 regardless of locale (`03` §6, verified by Arabic-Indic tests).
- Locale radix and grouping applied to all result rendering (CLI and search provider included; CLI output uses the locale radix of the invoking environment).
- Window title "Calculator" translated; "(Nightly) " prefix untranslated.

## 7. Right-to-left and Unicode behavior [V]

- Display and keypads forced LTR even under RTL locales (expressions read left-to-right).
- Converter value fields: LTR direction; under RTL widget direction they are right-justified.
- History entry texts and popover item labels are prefixed with U+200E (LTR mark) to keep rendering stable; tooltips likewise.
- Programming-mode base label starts with U+200E.
- Unicode variants accepted on input: `−`/`–` minus, `×`, `÷`, `∕`, `≪≫⋙`, sup/sub digits, vulgar fractions, DMS marks (`°`, `′`/`'`/`’`, `″`/`"`/`”`), fullwidth hex digits (via GLib digit classification).

## 8. Accessibility [V]

- Status/error text changes announced (medium priority); results announced after solving with the `=` button.
- Name entries in variable/function popovers set the a11y `INVALID` state on invalid names (v51).
- Keyboard-only operation is complete (all features reachable; exceptions documented: financial dialogs and character dialog are pointer/keyboard hybrids; conversion keywords typed).
- Shortcuts window (Ctrl+question) enumerates all bindings (verified against the original's shortcut definitions; traceability in doc 01).
- Styles: high-contrast and dark variants supported; history uses card styling; error state uses the "error" style class on the display container; bit panel uses accent/dimmed classes for float exponent bits.

## 9. Interchange formats (clipboard) [V]

- Copy from display: plain text, separators stripped, locale radix preserved.
- Copy from converter: plain text, locale thousands separator stripped.
- Search provider result: "clipboardText" metadata carries the plain result string; notification "Result copied" / "Result was copied successfully" on activation.
- No rich-text or specialized MIME types are offered.
