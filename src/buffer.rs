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
}

impl Line {
    pub fn from_text(s: &str) -> Self {
        Line {
            chars: s.chars().collect(),
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

    /// Display width of chars[..col].
    pub fn width_before(&self, col: usize) -> usize {
        self.chars[..col.min(self.chars.len())]
            .iter()
            .map(|c| c.width().unwrap_or(0))
            .sum()
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
        let mut lines: Vec<Line> = expanded.lines().map(Line::from_text).collect();
        while lines.len() > 1 && lines.last().is_some_and(|l| l.is_blank()) {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(Line { chars: Vec::new() });
        }
        Buffer { lines }
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

    /// Charwise text between two positions, both endpoints inclusive.
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
        for row in a.row + 1..b.row {
            out.push('\n');
            out.push_str(&self.lines[row].text());
        }
        out.push('\n');
        let last = self.line(b.row);
        out.extend(last.chars[..(b.col + 1).min(last.len())].iter());
        out
    }

    /// Linewise text for an inclusive row range, with a trailing newline.
    pub fn slice_lines(&self, r1: usize, r2: usize) -> String {
        let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
        let mut out = String::new();
        for row in r1..=r2.min(self.last_row()) {
            out.push_str(&self.lines[row].text());
            out.push('\n');
        }
        out
    }
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
