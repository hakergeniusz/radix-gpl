//! Display area logic (doc 06 §4, doc 05 §2–3, doc 02 §11): a multi-line
//! editable text view with key substitutions, superscript/subscript modes,
//! a bold answer region, undo/redo, and canonicalization before solving.

use gtk::prelude::*;
use gtk::TextBuffer;
use radix_core::serializer::Serializer;

/// A display snapshot for undo/redo.
#[derive(Clone, PartialEq)]
struct Snapshot {
    text: String,
    cursor: i32, // byte offset
}

/// Insertion modes: normal, superscript (↑n), subscript (↓n) — digit
/// keys insert their digit glyph while a mode is active; any non-digit
/// insertion returns to normal (doc 06 §9.1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InsertMode {
    Normal,
    Superscript,
    Subscript,
}

pub struct Display {
    pub buffer: TextBuffer,
    /// Byte range of the committed answer region, if the text is showing
    /// one (bold tag; canonicalizes to `_ `, doc 02 §11).
    answer_range: Option<(i32, i32)>,
    answer_value: Option<radix_core::Number>,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    insert_mode: InsertMode,
    /// Throttling: undo snapshots coalesce rapid typing of one token.
    last_snapshot: Option<Snapshot>,
}

const BOLD_TAG: &str = "answer";

impl Display {
    pub fn new() -> Self {
        let buffer = TextBuffer::default();
        let tag = gtk::TextTag::new(Some(BOLD_TAG));
        tag.set_weight(700); // pango::Weight::Bold
        buffer.tag_table().add(&tag);
        Display {
            buffer,
            answer_range: None,
            answer_value: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            insert_mode: InsertMode::Normal,
            last_snapshot: None,
        }
    }

    pub fn insert_mode(&self) -> InsertMode {
        self.insert_mode
    }

    pub fn set_insert_mode(&mut self, m: InsertMode) {
        self.insert_mode = m;
    }

    fn cursor_offset(buffer: &TextBuffer) -> i32 {
        buffer.cursor_position()
    }

    fn push_undo(&mut self) {
        let snap = Snapshot {
            text: self.text(),
            cursor: Self::cursor_offset(&self.buffer),
        };
        if self.last_snapshot.as_ref() != Some(&snap) {
            self.undo_stack.push(snap);
            self.redo_stack.clear();
            if self.undo_stack.len() > 200 {
                self.undo_stack.remove(0);
            }
        }
        self.last_snapshot = None;
    }

    /// Snapshot before an upcoming change (call before mutating).
    fn snapshot(&mut self) {
        let snap = Snapshot {
            text: self.text(),
            cursor: Self::cursor_offset(&self.buffer),
        };
        // Coalesce consecutive single-character edits within one second of
        // the same kind into one undo step (feels like upstream's editor).
        let coalesce = self
            .last_snapshot
            .as_ref()
            .map(|prev| {
                let cur = &snap.text;
                cur.len() > prev.text.len()
                    && cur.starts_with(&prev.text[..prev.text.len().saturating_sub(0)])
            })
            .unwrap_or(false);
        if !coalesce {
            self.undo_stack.push(snap);
            self.redo_stack.clear();
            if self.undo_stack.len() > 200 {
                self.undo_stack.remove(0);
            }
        }
    }

    /// Undo/redo stack emptiness for header-button sensitivity (doc 13 §3.2).
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        // Current state first, then pop.
        let cur = Snapshot {
            text: self.text(),
            cursor: Self::cursor_offset(&self.buffer),
        };
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(cur);
        self.apply_snapshot(&prev);
        true
    }

    pub fn redo(&mut self) -> bool {
        let cur = Snapshot {
            text: self.text(),
            cursor: Self::cursor_offset(&self.buffer),
        };
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(cur);
        self.apply_snapshot(&next);
        true
    }

    fn apply_snapshot(&mut self, snap: &Snapshot) {
        self.answer_range = None; // editing invalidates the answer region
        self.buffer.set_text(snap.text.as_str());
        let mut it = self.buffer.iter_at_offset(snap.cursor);
        self.buffer.place_cursor(&it);
        let _ = &mut it;
    }

    pub fn text(&self) -> String {
        let (s, e) = self.buffer.bounds();
        self.buffer.text(&s, &e, true).to_string()
    }

    pub fn set_text(&mut self, text: &str) {
        self.push_undo();
        self.answer_range = None;
        self.answer_value = None;
        self.buffer.set_text(text);
        self.buffer.place_cursor(&self.buffer.end_iter());
    }

    pub fn clear(&mut self) {
        self.push_undo();
        self.answer_range = None;
        self.answer_value = None;
        self.buffer.set_text("");
        self.last_snapshot = None;
    }

    pub fn answer_value(&self) -> Option<&radix_core::Number> {
        self.answer_value.as_ref()
    }

    pub fn has_answer(&self) -> bool {
        self.answer_range.is_some()
    }

    /// Selection bounds as (start, end) char offsets, if any.
    pub fn selection_bounds(&self) -> Option<(i32, i32)> {
        self.buffer
            .selection_bounds()
            .map(|(s, e)| (s.offset(), e.offset()))
    }

    /// Remove trailing spaces before the cursor (super/subscript digit
    /// inserts, doc 06 §9.1).
    pub fn strip_trailing_spaces(&mut self) {
        if self.selection_bounds().is_some() {
            return;
        }
        let cursor = self.buffer.cursor_position();
        let chars: Vec<char> = self.text().chars().collect();
        let mut n = 0usize;
        while (cursor as usize) > n
            && n < chars.len()
            && chars.get(cursor as usize - n - 1) == Some(&' ')
        {
            n += 1;
        }
        if n > 0 {
            let mut s = self.buffer.iter_at_offset(cursor - n as i32);
            let mut e = self.buffer.iter_at_offset(cursor);
            self.buffer.delete(&mut s, &mut e);
        }
    }

    /// Show a solved result: the whole text becomes the bold answer region
    /// (doc 05 §3). Cursor placed at the end.
    pub fn show_result(&mut self, value: &radix_core::Number, ser: &Serializer) {
        self.push_undo();
        self.answer_range = None;
        let rendered = ser.serialize(value).unwrap_or_default();
        self.buffer.set_text(&rendered);
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.apply_tag_by_name(BOLD_TAG, &start, &end);
        self.answer_range = Some((0, rendered.len() as i32));
        self.answer_value = Some(value.clone());
        self.buffer.place_cursor(&end);
        self.last_snapshot = None;
    }

    /// Re-render an existing answer (settings change: accuracy, format,
    /// grouping — doc 06 §14). No-ops when no answer is displayed.
    pub fn rerender_answer(&mut self, ser: &Serializer) {
        if let (Some(_), Some(v)) = (self.answer_range, self.answer_value.clone()) {
            let rendered = ser.serialize(&v).unwrap_or_default();
            self.buffer.set_text(&rendered);
            let start = self.buffer.start_iter();
            let end = self.buffer.end_iter();
            self.buffer.apply_tag_by_name(BOLD_TAG, &start, &end);
            self.answer_range = Some((0, rendered.len() as i32));
            self.buffer.place_cursor(&end);
        }
    }

    /// Insert text at the cursor, replacing the selection. Digits honor the
    /// super/subscript mode; any non-digit resets it (doc 06 §9.1).
    pub fn insert(&mut self, s: &str, digit: bool) {
        self.snapshot();
        if self.answer_range.is_some() {
            // Editing inside/before the answer region unmarks it; typing a
            // digit or radix at the very end starts a fresh calculation
            // (doc 05 §3).
            let at_end =
                Self::cursor_offset(&self.buffer) >= self.answer_range.map(|(_, e)| e).unwrap_or(0);
            if at_end && (digit || s == "." || s == ",") {
                self.answer_range = None;
                self.answer_value = None;
                self.buffer.set_text("");
            } else if digit {
                self.unmark_answer();
            } else {
                // Operator after a result: the answer stays marked and is
                // canonicalized to `_ ` (doc 05 §3).
            }
        }
        let mut s = s;
        let owned;
        if digit {
            match self.insert_mode {
                InsertMode::Normal => {}
                InsertMode::Superscript => {
                    owned = superscript_text(s);
                    s = &owned;
                }
                InsertMode::Subscript => {
                    owned = subscript_text(s);
                    s = &owned;
                }
            }
        } else {
            self.insert_mode = InsertMode::Normal;
        }
        // Strip thousands separators (U+202F) when pasting/inserting.
        let clean: String = s.chars().filter(|c| *c != '\u{202F}').collect();
        self.buffer.delete_selection(true, true);
        self.buffer.insert_at_cursor(&clean);
    }

    fn unmark_answer(&mut self) {
        if let Some((a, b)) = self.answer_range {
            let start = self.buffer.iter_at_offset(a);
            let end = self.buffer.iter_at_offset(b);
            self.buffer.remove_tag_by_name(BOLD_TAG, &start, &end);
        }
        self.answer_range = None;
        self.answer_value = None;
    }

    pub fn backspace(&mut self) {
        self.snapshot();
        self.answer_range = None;
        self.answer_value = None;
        let mut ins = self.buffer.iter_at_mark(&self.buffer.get_insert());
        if ins.offset() > 0 {
            let mut start = ins;
            // Delete one grapheme-ish char (iter_backward_char is fine for
            // the calculator's composed Unicode operators).
            if !start.backward_char() {
                return;
            }
            self.buffer.delete(&mut start, &mut ins);
        }
    }

    /// Canonicalize the display text for the parser (doc 02 §11):
    /// answer region → `_ `, narrow-space separators dropped, radix kept.
    pub fn canonicalize(&self) -> String {
        let mut text = self.text();
        if let Some((a, b)) = self.answer_range {
            let (a, b) = (a as usize, b as usize);
            if b <= text.len() {
                text = format!("{}_ {}", &text[..a], &text[b..]);
            }
        }
        let text: String = text.chars().filter(|c| *c != '\u{202F}').collect();
        text
    }

    /// Select the given byte range (error-token highlighting, doc 06 §4).
    pub fn select_range(&mut self, start: usize, end: usize) {
        let text = self.text();
        let start = char_to_offset(&text, start);
        let end = char_to_offset(&text, end);
        let si = self.buffer.iter_at_offset(start);
        let ei = self.buffer.iter_at_offset(end);
        self.buffer.select_range(&si, &ei);
    }

    /// Whether the current text is a bare variable name or function header
    /// (`=` then inserts a literal `=`, doc 05 §1.2).
    pub fn is_name_or_header(&self) -> bool {
        let t = self.text();
        let t = t.trim();
        if t.is_empty() {
            return false;
        }
        let alpha = |c: char| c.is_alphabetic() || c == '_';
        if t.chars().all(alpha) {
            return true;
        }
        // Function header: name(params) with an empty or missing tail after
        // the closing paren.
        if let Some(close) = t.rfind(')') {
            if t[close + 1..].trim().is_empty() {
                if let Some(open) = t.find('(') {
                    if open > 0 && t[..open].chars().all(alpha) {
                        let inner = &t[open + 1..close];
                        return inner
                            .split(';')
                            .all(|p| p.trim().chars().all(alpha) || p.trim().is_empty());
                    }
                }
            }
        }
        false
    }
}

fn char_to_offset(text: &str, char_index: usize) -> i32 {
    text.char_indices()
        .nth(char_index)
        .map(|(b, _)| b as i32)
        .unwrap_or(text.len() as i32)
}

pub fn superscript_text(digits: &str) -> String {
    digits
        .chars()
        .map(radix_core::serializer::superscript_digit)
        .collect()
}

pub fn subscript_text(digits: &str) -> String {
    digits
        .chars()
        .map(radix_core::serializer::subscript_digit)
        .collect()
}
