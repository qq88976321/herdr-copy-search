use crate::ansi;
use unicode_width::UnicodeWidthChar;

/// A cursor position in the buffer. `col` is a char index, not a display
/// column; wide (CJK) chars occupy one col here but two display cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

pub struct Line {
    pub chars: Vec<char>,
    /// Per-char source styles; empty = all default (from_text path).
    pub styles: Vec<ansi::Style>,
    /// True when this row soft-wraps into the next row (no real newline
    /// between them); slicing joins such rows without a line break.
    pub soft_wrap: bool,
}

impl Line {
    pub fn from_text(s: &str) -> Self {
        Line {
            chars: s.chars().collect(),
            styles: Vec::new(),
            soft_wrap: false,
        }
    }

    fn from_styled(l: ansi::StyledLine) -> Self {
        Line {
            chars: l.chars,
            styles: l.styles,
            soft_wrap: false,
        }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn is_blank(&self) -> bool {
        self.chars.iter().all(|c| c.is_whitespace())
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Source style of the char at `col`; default past end of line.
    pub fn style_at(&self, col: usize) -> ansi::Style {
        self.styles.get(col).copied().unwrap_or_default()
    }

    /// Display width of chars[..col].
    pub fn width_before(&self, col: usize) -> usize {
        self.chars[..col.min(self.chars.len())]
            .iter()
            .map(|c| c.width().unwrap_or(0))
            .sum()
    }

    /// Display cell width of the char at `col`, at least 1 so the cursor
    /// always occupies a cell (also past end of line).
    pub fn cell_width(&self, col: usize) -> usize {
        self.chars
            .get(col)
            .and_then(|c| c.width())
            .unwrap_or(1)
            .max(1)
    }

    /// Char index whose display cell range contains column x (with the
    /// given horizontal offset already subtracted by the caller).
    pub fn col_at_x(&self, x: usize) -> usize {
        let mut acc = 0;
        for (i, c) in self.chars.iter().enumerate() {
            let w = c.width().unwrap_or(0).max(1);
            if x < acc + w {
                return i;
            }
            acc += w;
        }
        self.len().saturating_sub(1)
    }
}

pub struct Buffer {
    pub lines: Vec<Line>,
}

impl Buffer {
    pub fn from_text(text: &str) -> Self {
        let expanded = text.replace('\t', "    ");
        Self::finalize(expanded.lines().map(Line::from_text).collect())
    }

    /// Plain or SGR-styled text; styles are kept per char, other escape
    /// sequences are stripped. Used for --input files and piped stdin.
    pub fn from_ansi(text: &str) -> Self {
        Self::finalize(
            ansi::parse(text)
                .into_iter()
                .map(Line::from_styled)
                .collect(),
        )
    }

    /// Compose a styled buffer from herdr's two reliable pane reads:
    /// `wrapped_ansi` (recent, --format ansi: styled rows wrapped at the
    /// pane width) and `unwrapped` (recent-unwrapped, --format text:
    /// logical lines). Rows are the wrapped ones, so the view matches
    /// the frozen pane; the unwrapped text aligns them to mark
    /// soft_wrap flags so yanks rejoin wrapped long lines. On any
    /// alignment mismatch the flags safely degrade to hard breaks.
    pub fn from_reads(unwrapped: &str, wrapped_ansi: &str) -> Self {
        let mut lines: Vec<Line> = ansi::parse(wrapped_ansi)
            .into_iter()
            .map(Line::from_styled)
            .collect();
        if !mark_soft_wraps(&mut lines, unwrapped) {
            for line in &mut lines {
                line.soft_wrap = false;
            }
        }
        Self::finalize(lines)
    }

    fn finalize(mut lines: Vec<Line>) -> Self {
        while lines.len() > 1 && lines.last().is_some_and(|l| l.is_blank()) {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(Line::from_text(""));
        }
        Buffer { lines }
    }

    /// Logical text: rows joined with newlines, except soft-wrapped
    /// rows which continue without a break. Feeds extract tokenization.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push_str(&line.text());
            if !line.soft_wrap {
                out.push('\n');
            }
        }
        out
    }

    pub fn last_row(&self) -> usize {
        self.lines.len() - 1
    }

    pub fn line(&self, row: usize) -> &Line {
        &self.lines[row]
    }

    /// Max cursor col for a row; the cursor sits on a char (vi style).
    pub fn max_col(&self, row: usize) -> usize {
        self.line(row).len().saturating_sub(1)
    }

    pub fn clamp(&self, p: Pos) -> Pos {
        let row = p.row.min(self.last_row());
        Pos {
            row,
            col: p.col.min(self.max_col(row)),
        }
    }

    pub fn first_non_blank(&self, row: usize) -> usize {
        self.line(row)
            .chars
            .iter()
            .position(|c| !c.is_whitespace())
            .unwrap_or(0)
    }

    /// Newline or nothing after `row` in yanked text, depending on wrap.
    fn break_after(&self, row: usize) -> &'static str {
        if self.lines[row].soft_wrap {
            ""
        } else {
            "\n"
        }
    }

    /// Charwise text between two positions, both endpoints inclusive.
    /// Soft-wrapped rows are joined without a newline.
    pub fn slice_inclusive(&self, a: Pos, b: Pos) -> String {
        let (a, b) = if (a.row, a.col) <= (b.row, b.col) {
            (a, b)
        } else {
            (b, a)
        };
        if a.row == b.row {
            let line = self.line(a.row);
            let end = (b.col + 1).min(line.len());
            return line.chars[a.col.min(line.len())..end].iter().collect();
        }
        let mut out = String::new();
        let first = self.line(a.row);
        out.extend(first.chars[a.col.min(first.len())..].iter());
        out.push_str(self.break_after(a.row));
        for row in a.row + 1..b.row {
            out.push_str(&self.lines[row].text());
            out.push_str(self.break_after(row));
        }
        let last = self.line(b.row);
        out.extend(last.chars[..(b.col + 1).min(last.len())].iter());
        out
    }

    /// Linewise text for an inclusive row range, with a trailing newline.
    /// Soft-wrapped rows are joined without a newline.
    pub fn slice_lines(&self, r1: usize, r2: usize) -> String {
        let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
        let mut out = String::new();
        for row in r1..=r2.min(self.last_row()) {
            out.push_str(&self.lines[row].text());
            if row == r2.min(self.last_row()) {
                out.push('\n');
            } else {
                out.push_str(self.break_after(row));
            }
        }
        out
    }
}

/// Align wrapped rows to logical lines, newest first, and set soft_wrap
/// on every row that continues into the next. The wrapped read covers
/// fewer logical lines than the unwrapped one (both are tail-capped by
/// the server), so alignment walks both from the bottom; the oldest
/// wrapped rows may form only a suffix of their logical line. Returns
/// false when the texts do not line up (e.g. the pane changed between
/// the two reads).
fn mark_soft_wraps(lines: &mut [Line], unwrapped: &str) -> bool {
    let logical: Vec<Vec<char>> = unwrapped.lines().map(|l| l.chars().collect()).collect();
    let mut end = lines.len();
    for want in logical.iter().rev() {
        if end == 0 {
            return true;
        }
        let mut start = end;
        loop {
            if start == 0 {
                // Oldest rows may be a mid-line tail of the logical line.
                if !rows_match(&lines[start..end], want, true) {
                    return false;
                }
                break;
            }
            start -= 1;
            let total: usize = lines[start..end].iter().map(|l| l.len()).sum();
            if total < want.len() {
                continue;
            }
            if !rows_match(&lines[start..end], want, false) {
                return false;
            }
            break;
        }
        for line in &mut lines[start..end.saturating_sub(1)] {
            line.soft_wrap = true;
        }
        end = start;
    }
    true
}

/// Do `rows` concatenate to `want` (or, with `suffix`, to a tail of it)?
fn rows_match(rows: &[Line], want: &[char], suffix: bool) -> bool {
    let total: usize = rows.iter().map(|l| l.len()).sum();
    if suffix {
        if total > want.len() {
            return false;
        }
    } else if total != want.len() {
        return false;
    }
    let mut pos = want.len() - total;
    for row in rows {
        if want[pos..pos + row.len()] != row.chars[..] {
            return false;
        }
        pos += row.len();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_text_trims_trailing_blank_lines() {
        let b = Buffer::from_text("a\nb\n\n\n");
        assert_eq!(b.lines.len(), 2);
    }

    #[test]
    fn from_ansi_plain_matches_from_text() {
        let b = Buffer::from_ansi("a\tb\nc\n\n");
        assert_eq!(b.lines.len(), 2);
        assert_eq!(b.lines[0].text(), "a    b");
        assert!(b.lines[0].styles.is_empty());
    }

    #[test]
    fn from_ansi_keeps_styles() {
        let b = Buffer::from_ansi("\x1b[31mred\x1b[0m ok");
        assert_eq!(b.lines[0].text(), "red ok");
        assert_eq!(b.lines[0].style_at(0).fg, ansi::Color::Indexed(1));
        assert_eq!(b.lines[0].style_at(4), ansi::Style::default());
    }

    #[test]
    fn from_reads_marks_wrapped_rows_and_joins_yanks() {
        // Logical: one 8-char line wrapped at 4 cols, then a short line.
        let unwrapped = "abcdefgh\nok";
        let wrapped = "abcd\nefgh\nok";
        let b = Buffer::from_reads(unwrapped, wrapped);
        assert_eq!(b.lines.len(), 3);
        assert!(b.lines[0].soft_wrap);
        assert!(!b.lines[1].soft_wrap);
        assert!(!b.lines[2].soft_wrap);
        assert_eq!(b.slice_lines(0, 1), "abcdefgh\n");
        assert_eq!(
            b.slice_inclusive(Pos { row: 0, col: 2 }, Pos { row: 1, col: 1 }),
            "cdef"
        );
        assert_eq!(b.text(), "abcdefgh\nok\n");
    }

    #[test]
    fn from_reads_accepts_partial_oldest_line() {
        // The wrapped read starts mid-way through the oldest logical line.
        let unwrapped = "0123456789abc\nok";
        let wrapped = "89ab\nc\nok";
        let b = Buffer::from_reads(unwrapped, wrapped);
        assert!(b.lines[0].soft_wrap);
        assert!(!b.lines[1].soft_wrap);
        assert_eq!(b.slice_lines(0, 1), "89abc\n");
    }

    #[test]
    fn from_reads_covers_fewer_logical_lines_than_unwrapped() {
        // Older unwrapped lines beyond the wrapped read are ignored.
        let unwrapped = "old-history\nabcd\nok";
        let wrapped = "abcd\nok";
        let b = Buffer::from_reads(unwrapped, wrapped);
        assert_eq!(b.lines.len(), 2);
        assert!(!b.lines[0].soft_wrap);
    }

    #[test]
    fn from_reads_mismatch_degrades_to_hard_breaks() {
        let b = Buffer::from_reads("something\nelse", "abcd\nefgh");
        assert!(b.lines.iter().all(|l| !l.soft_wrap));
        assert_eq!(b.slice_lines(0, 1), "abcd\nefgh\n");
    }

    #[test]
    fn from_reads_handles_empty_lines() {
        let unwrapped = "one\n\ntwo";
        let wrapped = "one\n\ntwo";
        let b = Buffer::from_reads(unwrapped, wrapped);
        assert_eq!(b.lines.len(), 3);
        assert!(b.lines.iter().all(|l| !l.soft_wrap));
    }

    #[test]
    fn slice_inclusive_single_line() {
        let b = Buffer::from_text("hello world");
        let s = b.slice_inclusive(Pos { row: 0, col: 6 }, Pos { row: 0, col: 10 });
        assert_eq!(s, "world");
    }

    #[test]
    fn slice_inclusive_multi_line_and_swapped() {
        let b = Buffer::from_text("one\ntwo\nthree");
        let s = b.slice_inclusive(Pos { row: 2, col: 2 }, Pos { row: 0, col: 1 });
        assert_eq!(s, "ne\ntwo\nthr");
    }

    #[test]
    fn slice_lines_has_trailing_newline() {
        let b = Buffer::from_text("one\ntwo");
        assert_eq!(b.slice_lines(1, 0), "one\ntwo\n");
    }

    #[test]
    fn col_at_x_handles_wide_chars() {
        let line = Line::from_text("a\u{4e2d}b");
        assert_eq!(line.col_at_x(0), 0);
        assert_eq!(line.col_at_x(1), 1);
        assert_eq!(line.col_at_x(2), 1);
        assert_eq!(line.col_at_x(3), 2);
        assert_eq!(line.width_before(2), 3);
    }
}
