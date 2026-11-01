# 05 — Evaluation and Error Semantics

**Scope:** the observable evaluation lifecycle: when evaluation happens, how incomplete expressions behave, how errors are surfaced and recovered from, and the interaction with history/undo. The evaluation *model* is specified here; the internal threading is described only where observable.
**Evidence:** direct source analysis of the evaluation lifecycle, error paths, and evaluation limits, plus the application shell and its test suite. Confidence: **V** / **S** / **I** as in `02`.

---

## 1. Evaluation lifecycle [V]

1. The user's display text is continuously present; **no live re-evaluation of partial input is displayed** — the display keeps the last committed result or the user's text. (The "live evaluation" upstream historically documented is not present in this version: results appear only on commit. Status line stays empty during editing.)
2. **Commit** happens on: pressing `=`, keypad `=` button, or Enter (Return/KP Enter). Exception: if the current canonical expression is exactly a variable name or a function-definition header (`name(params)` possibly with trailing spaces), `=` inserts a literal `=` character instead of solving [V — enables `f(x)=` and `name=` prefixes].
3. On commit the display text is canonicalized (`02` §11) and evaluated **asynchronously** (worker thread). During the solve: keypresses into the display are ignored; if the solve takes longer than 100 ms a spinner appears and the status shows "Calculating" [V]. Completion is polled every 50 ms.
4. On success the display is replaced by the serialized result (see `07`), the result is registered as the **answer** (`_`), and an entry `(expression, result)` is appended to history. The expression stored in history is the canonical equation with `\cdot` replaced by `×`.
5. **Repeating the last operation [V]:** pressing `=` (or Enter) while the display shows a committed result re-solves the previous equation with its last binary operator and right operand re-appended: `5+3=` → 8; `=` again → 11; `=` again → 14. This works for `+ − × ÷` and shifts (operators marked repeatable; implicit multiplication is NOT repeatable). If the result was not produced by a real answer (e.g. user typed `_ ` manually), the repeat is skipped.
6. **Assignment** (`x=5`) stores the variable and displays 5. **Function definition** (`f(x)=x+1`) stores the function and displays 0.

## 2. Incomplete and empty input [V]

- Empty display: solve is a no-op.
- Display showing a sign/radix only (`+`, `-`, `−`, `.`, `+.`, …): treated as "not a number" — solve still attempts parse (fails with INVALID → status shows "Malformed expression"); bit panel and number-dependent features treat it as empty.
- Trailing operators (`1+`) → parse error at commit; the display is left as-is (error shown, see §4).
- While typing, thousands separators and smart-bracket behavior are applied by the editor (see `06`), which can make text that is not yet a valid expression; errors at that stage are not displayed.

## 3. Result display state [V]

- After a successful solve the display shows the result **as editable text** with the answer region marked (bold). The region remembers it is the answer.
- Typing a digit or radix **at the end** of a displayed result clears the display first (starting a fresh calculation) [V]. Typing an operator continues from the answer (the answer becomes the `_` operand) [V].
- Inserting/deleting inside or before the answer region clears the answer marking (the text becomes a plain number literal).
- Editing actions replace the display with re-formatted values when settings change (accuracy/format/grouping) — the answer region is re-rendered from the stored value (`07` §5).

## 4. Error surfacing [V]

Errors appear in a status/info line below the display:

| Condition | Message (English) |
|---|---|
| Syntax error (`INVALID`), and all budget/timeouts in-app | "Malformed expression" |
| Unknown variable | "Unknown variable “%s”" |
| Unknown function | "Function “%s” is not defined" |
| Unknown unit | "Unknown unit “%s”" |
| Unknown conversion (incompatible categories, bad base) | "Unknown conversion" |
| Currency without rate | "Exchange rate for “%s” is not available" |
| Math-level error with a message (`MP`) | the message itself (see `03`/`04` tables; e.g. "Division by zero is undefined", "Overflow error", "Underflow error", "Zero raised to zero is undefined", function domain messages, "Malformed expression" fallback when no token) |
| Factorize on non-integer | "Need an integer to factorize" |
| Store variable with no value | "No sane value to store" |
| Undo with empty stack | "No undo history" |
| Redo with empty stack | "No redo history" |
| While solving (>100 ms) | "Calculating" |

Additional behaviors [V]:
- The error **selects the offending token** in the display: the error token's start/end offsets are used to select the substring; the selection is adjusted to include auto-inserted thousands separators and to extend over missing parentheses (auto-closed brackets).
- The status line gets an "error" visual style; the message is announced to assistive technologies (medium priority).
- Non-tokenized errors (e.g. serializer display errors like "The result is too long to display in fixed format. Try other result formats") also appear in the status line.
- **Errors do not block anything:** the display text remains editable; the next commit re-evaluates from scratch; variables/functions defined before a failed expression remain defined. A failed commit does NOT create a history entry.

## 5. Evaluation limits [V — search provider only]

The search provider (and any budgeted caller) enforces: ≤256 input chars, ≤256 tokens, ≤256 output digits, ≤100000 evaluation steps, factorial ≤ 10000, power exponent ≤ 100000, modular exponent ≤ 1000000, permutation ≤ 10000, wall-clock deadline (50 ms). Breaching yields `RESOURCE_LIMIT`/`TIMED_OUT` with messages "Expression is too long", "Expression is too complex", "Result is too large", "Evaluation requires too many steps", "Exponent is too large", "Root is too large", "Factorial is too large", "Permutation is too large", "Modular exponent is too large", "Evaluation timed out". The main GUI does NOT impose these limits (no budget) — only the wall-clock "Calculating" indicator exists there.

## 6. Factorization [V]

The advanced-mode `a×b` button / `Ctrl+F` factorizes the currently displayed number (if the display shows a result, its value; else the parsed display value). Non-integer → status error. The result replaces the display as a product string (`2×3×5` style, `×` separators) and is NOT added to history. Runs on the worker thread (spinner for slow cases).

## 7. Undo/redo and history interaction [V]

- Every display edit (insert/delete, including programmatic ones) pushes an undo state capturing: full text, cursor, answer region, number mode, status. Undo/redo restore that state exactly (including the answer region and its value).
- Undo/redo are header-bar buttons (enabled/disabled by stack state) and Ctrl+Z / Ctrl+Shift+Z.
- History (the list above the display) is separate from undo: history entries are created only by successful solves (and restored from the persisted tape at startup). Clicking an entry's equation loads it into the display (as plain text, answer marking cleared); clicking an answer inserts that number at the cursor.
- Alt+Left / Alt+Right walk back/forward through history entries into the display; the in-progress text is remembered so returning forward restores it.
- Clearing: two distinct "clear" actions exist [V]:
  - **Clear display** — the `C` keypad button, or `Escape` with no modifiers while the display has focus: empties the display text and answer marking (history is untouched).
  - **Clear history** — the primary-menu item "Clear History", accelerator `Ctrl+Escape`: empties the history view and erases the persisted history tape (display is untouched).

## 8. Concurrency and re-entrancy [V]

- Solves run on worker threads; a solve in progress blocks new solves (ignored). Clearing the display during a solve cancels the pending result consumption.
- Copying during a solve is allowed; pasting is processed normally after.
- The GUI remains responsive during long computations (spinner shown).

## 9. Interaction with settings changes [V]

Changing accuracy / format / grouping / trailing zeroes / angle units / word size / base immediately re-renders the display (answer region re-serialized; separators re-applied) without re-evaluating. Angle-unit and word-size changes do not re-evaluate committed expressions; they affect the next evaluation.
