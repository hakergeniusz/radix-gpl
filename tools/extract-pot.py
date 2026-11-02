#!/usr/bin/env python3
"""Regenerate crates/radix-gui/data/locales/radix.pot (D33 skeleton).

Msgid sources — all English, the clean-room spec text:
  1. `crate::i18n::gettext("...")` literals in the radix-gui sources;
  2. unit display names: the second string literal of every `u!`
     invocation in radix-core/src/units.rs (the first is the category
     key, the rest are parser symbols — not translatable);
  3. category display names: the second element of CATEGORY_ORDER;
  4. function descriptions: `description: "..."` in functions.rs.

Symbol labels (unit parser keywords, function names, button glyphs) and
engine error messages are deliberately not collected. Re-run after UI
changes and commit the result:

    python3 tools/extract-pot.py
"""
import json
import pathlib
import re

root = pathlib.Path(__file__).resolve().parents[1]
out_path = root / "crates/radix-gui/data/locales/radix.pot"

HEADER = """msgid ""
msgstr ""
"Project-Id-Version: radix 51.1.2\\n"
"Report-Msgid-Bugs-To: https://github.com/hakergeniusz/radix/issues\\n"
"POT-Creation-Date: {date}\\n"
"MIME-Version: 1.0\\n"
"Content-Type: text/plain; charset=UTF-8\\n"
"Content-Transfer-Encoding: 8bit\\n"
"Plural-Forms: nplurals=INTEGER; plural=EXPRESSION;\\n"
"""

# (msgid, reference) pairs, kept in first-seen order per msgid.
entries: dict[str, list[str]] = {}


def add(msgid: str, ref: str) -> None:
    if not msgid:
        return
    entries.setdefault(msgid, []).append(ref)


def rel(p: pathlib.Path) -> str:
    return str(p.relative_to(root))


# 1. Wrapped GUI literals.
gui_re = re.compile(r'i18n::gettext\("((?:[^"\\]|\\.)*)"\)')
for src in sorted((root / "crates/radix-gui/src").rglob("*.rs")):
    for i, line in enumerate(src.read_text().splitlines(), 1):
        for m in gui_re.finditer(line):
            add(json.loads(f'"{m.group(1)}"'), f"{rel(src)}:{i}")

# 1b. Strings passed through a variable (translated at the display site):
#     MODE_NAMES labels and the Result Format tuples in win.rs.
win = root / "crates/radix-gui/src/win.rs"
wtext = win.read_text()
for m in re.finditer(
    r"MODE_NAMES: \[\(ButtonMode, &str\); 6\] = \[(.*?)\]", wtext, re.S
):
    for i, t in enumerate(re.finditer(r'ButtonMode::\w+,\s*"([^"]+)"', m.group(1))):
        line = wtext[: m.start()].count("\n") + i + 1
        add(t.group(1), f"{rel(win)}:{line}")
for m in re.finditer(r'\("(?:automatic|fixed|scientific|engineering)",\s*"([^"]+)"\)', wtext):
    line = wtext[: m.start()].count("\n") + 1
    add(m.group(1), f"{rel(win)}:{line}")

# 1c. More variable-backed literals (Codex P2 on PR #4): the base-name
#     tuples in keypad.rs (also the msgids for win.rs's base_name match
#     arms), the base list in converter.rs, the financial parameter
#     labels and the shortcut titles in dialogs.rs.
keypad = root / "crates/radix-gui/src/keypad.rs"
ktext = keypad.read_text()
for m in re.finditer(r"for \(name, base\) in \[(.*?)\]", ktext, re.S):
    for i, t in enumerate(re.finditer(r'\("([^"]+)",\s*\d+\)', m.group(1))):
        line = ktext[: m.start()].count("\n") + i + 1
        add(t.group(1), f"{rel(keypad)}:{line}")
conv = root / "crates/radix-gui/src/converter.rs"
ctext = conv.read_text()
for m in re.finditer(
    r'\["(?:Binary|Octal|Decimal|Hexadecimal)"(?:\s*,\s*"(?:Binary|Octal|Decimal|Hexadecimal)")+\]',
    ctext,
):
    for i, t in enumerate(re.finditer(r'"((?:Binary|Octal|Decimal|Hexadecimal))"', m.group(0))):
        line = ctext[: m.start()].count("\n") + 1
        add(t.group(1), f"{rel(conv)}:{line}")
dlg = root / "crates/radix-gui/src/dialogs.rs"
dtext = dlg.read_text()
for m in re.finditer(r"params:\s*&\[(.*?)\]", dtext, re.S):
    for t in re.finditer(r'"((?:[^"\\]|\\.)*)"', m.group(1)):
        line = dtext[: m.start()].count("\n") + 1
        add(json.loads(f'"{t.group(1)}"'), f"{rel(dlg)}:{line}")
for m in re.finditer(r"\[\(&str, &str\); \d+\] = \[(.*?)\];", dtext, re.S):
    for i, t in enumerate(re.finditer(r'\("([^"]+)",\s*"[^"]*"\)', m.group(1))):
        line = dtext[: m.start()].count("\n") + i + 1
        add(t.group(1), f"{rel(dlg)}:{line}")

# 2. Unit display names: second string literal inside each u!(...) call.
units = root / "crates/radix-core/src/units.rs"
text = units.read_text()
for m in re.finditer(r"u!\(", text):
    depth = 1
    i = m.end()
    while depth and i < len(text):
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
        i += 1
    block = text[m.end() : i - 1]
    strings = re.findall(r'"((?:[^"\\]|\\.)*)"', block)
    if len(strings) >= 2:
        line = text[: m.start()].count("\n") + 1
        add(json.loads(f'"{strings[1]}"'), f"{rel(units)}:{line}")

# 3. Category display names.
order = re.search(
    r"pub const CATEGORY_ORDER: &\[\(&str, &str\)\] = &\[(.*?)\];",
    text,
    re.S,
)
if order:
    for i, m in enumerate(
        re.finditer(r'\("([a-z-]+)", "([^"]+)"\)', order.group(1))
    ):
        add(m.group(2), f"{rel(units)}:{text[: text.find('CATEGORY_ORDER')].count(chr(10)) + 3 + i}")

# 4. Function descriptions.
funcs = root / "crates/radix-core/src/functions.rs"
for i, line in enumerate(funcs.read_text().splitlines(), 1):
    m = re.search(r'description: "((?:[^"\\]|\\.)*)"', line)
    if m:
        add(json.loads(f'"{m.group(1)}"'), f"{rel(funcs)}:{i}")

out = [HEADER.format(date=__import__("datetime").date.today().isoformat())]
for msgid in sorted(entries):
    for ref in entries[msgid]:
        out.append(f"#: {ref}")
    out.append(f'msgid "{msgid}"')
    out.append('msgstr ""')
    out.append("")
out_path.write_text("\n".join(out))
print(f"wrote {out_path.relative_to(root)}: {len(entries)} msgids")
