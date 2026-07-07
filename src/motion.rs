use crate::buffer::{Buffer, Pos};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Punct,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}

fn char_at(buf: &Buffer, p: Pos) -> Option<char> {
    buf.lines.get(p.row)?.chars.get(p.col).copied()
}

/// Line breaks and empty lines read as whitespace, like vi word motions.
fn class_at(buf: &Buffer, p: Pos) -> Class {
    char_at(buf, p).map(class).unwrap_or(Class::Space)
}

/// Steps through a virtual position at col == len (the newline), which
/// reads as whitespace via class_at, so word runs break at line ends.
fn advance(buf: &Buffer, p: Pos) -> Option<Pos> {
    if p.col < buf.line(p.row).len() {
        return Some(Pos {
            row: p.row,
            col: p.col + 1,
        });
    }
    if p.row < buf.last_row() {
        return Some(Pos {
            row: p.row + 1,
            col: 0,
        });
    }
    None
}

fn retreat(buf: &Buffer, p: Pos) -> Option<Pos> {
    if p.col > 0 {
        return Some(Pos {
            row: p.row,
            col: p.col - 1,
        });
    }
    if p.row > 0 {
        let row = p.row - 1;
        return Some(Pos {
            row,
            col: buf.max_col(row),
        });
    }
    None
}

/// w: start of the next word.
pub fn word_forward(buf: &Buffer, p: Pos) -> Pos {
    let mut cur = p;
    let start = class_at(buf, cur);
    if start != Class::Space {
        loop {
            match advance(buf, cur) {
                Some(n) => {
                    let moved_class = class_at(buf, n);
                    cur = n;
                    if moved_class != start {
                        break;
                    }
                }
                None => return buf.clamp(cur),
            }
        }
    }
    while class_at(buf, cur) == Class::Space {
        match advance(buf, cur) {
            Some(n) => cur = n,
            None => return buf.clamp(cur),
        }
    }
    cur
}

/// b: start of the current or previous word.
pub fn word_backward(buf: &Buffer, p: Pos) -> Pos {
    let mut cur = match retreat(buf, p) {
        Some(n) => n,
        None => return p,
    };
    while class_at(buf, cur) == Class::Space {
        match retreat(buf, cur) {
            Some(n) => cur = n,
            None => return cur,
        }
    }
    let cls = class_at(buf, cur);
    loop {
        match retreat(buf, cur) {
            Some(n) => {
                if class_at(buf, n) != cls {
                    return cur;
                }
                cur = n;
            }
            None => return cur,
        }
    }
}

/// e: end of the current or next word.
pub fn word_end(buf: &Buffer, p: Pos) -> Pos {
    let mut cur = match advance(buf, p) {
        Some(n) => n,
        None => return p,
    };
    while class_at(buf, cur) == Class::Space {
        match advance(buf, cur) {
            Some(n) => cur = n,
            None => return buf.clamp(cur),
        }
    }
    let cls = class_at(buf, cur);
    loop {
        match advance(buf, cur) {
            Some(n) => {
                if class_at(buf, n) != cls {
                    return cur;
                }
                cur = n;
            }
            None => return cur,
        }
    }
}

/// Inclusive char-index run of the class at `col` within `chars`.
/// `chars` must be non-empty; `col` is clamped to the last index.
fn run_bounds(chars: &[char], col: usize, cls: impl Fn(char) -> u8) -> (usize, usize) {
    let last = chars.len() - 1;
    let col = col.min(last);
    let target = cls(chars[col]);
    let mut s = col;
    while s > 0 && cls(chars[s - 1]) == target {
        s -= 1;
    }
    let mut e = col;
    while e < last && cls(chars[e + 1]) == target {
        e += 1;
    }
    (s, e)
}

/// vi word/WORD text object around `p`, returning the inclusive
/// (start, end) positions on p's row. `around` (aw/aW) extends over
/// adjacent whitespace (trailing preferred, else leading; on a space
/// run it takes the following word instead); `big` (iW/aW) treats any
/// non-whitespace run as one WORD. Operates within the single row.
pub fn word_object(buf: &Buffer, p: Pos, around: bool, big: bool) -> (Pos, Pos) {
    let chars = &buf.line(p.row).chars;
    if chars.is_empty() {
        let z = Pos { row: p.row, col: 0 };
        return (z, z);
    }
    // 0 = whitespace; small words split word/punct (1/2), WORDs do not.
    let small = |c: char| match class(c) {
        Class::Space => 0u8,
        Class::Word => 1,
        Class::Punct => 2,
    };
    let big_cls = |c: char| u8::from(class(c) != Class::Space);
    let (mut s, mut e) = if big {
        run_bounds(chars, p.col, big_cls)
    } else {
        run_bounds(chars, p.col, small)
    };
    if around {
        let last = chars.len() - 1;
        let on_space = class(chars[p.col.min(last)]) == Class::Space;
        if on_space {
            // Whitespace object: take the run of the word that follows.
            if e < last {
                let (_, e2) = if big {
                    run_bounds(chars, e + 1, big_cls)
                } else {
                    run_bounds(chars, e + 1, small)
                };
                e = e2;
            }
        } else if e < last && class(chars[e + 1]) == Class::Space {
            // Prefer trailing whitespace.
            while e < last && class(chars[e + 1]) == Class::Space {
                e += 1;
            }
        } else {
            // No trailing whitespace: fall back to leading whitespace.
            while s > 0 && class(chars[s - 1]) == Class::Space {
                s -= 1;
            }
        }
    }
    (Pos { row: p.row, col: s }, Pos { row: p.row, col: e })
}

/// }: next blank line (or the last row).
pub fn para_forward(buf: &Buffer, row: usize) -> usize {
    let mut r = row;
    while r < buf.last_row() && buf.line(r).is_blank() {
        r += 1;
    }
    while r < buf.last_row() && !buf.line(r).is_blank() {
        r += 1;
    }
    r
}

/// {: previous blank line (or row 0).
pub fn para_backward(buf: &Buffer, row: usize) -> usize {
    let mut r = row;
    while r > 0 && buf.line(r).is_blank() {
        r -= 1;
    }
    while r > 0 && !buf.line(r).is_blank() {
        r -= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(row: usize, col: usize) -> Pos {
        Pos { row, col }
    }

    #[test]
    fn word_forward_walks_runs_and_lines() {
        let b = Buffer::from_text("foo  bar.baz\nqux");
        assert_eq!(word_forward(&b, p(0, 0)), p(0, 5));
        assert_eq!(word_forward(&b, p(0, 5)), p(0, 8));
        assert_eq!(word_forward(&b, p(0, 8)), p(0, 9));
        assert_eq!(word_forward(&b, p(0, 9)), p(1, 0));
        assert_eq!(word_forward(&b, p(1, 0)), p(1, 2));
    }

    #[test]
    fn word_backward_walks_runs_and_lines() {
        let b = Buffer::from_text("foo  bar.baz\nqux");
        assert_eq!(word_backward(&b, p(1, 0)), p(0, 9));
        assert_eq!(word_backward(&b, p(0, 9)), p(0, 8));
        assert_eq!(word_backward(&b, p(0, 8)), p(0, 5));
        assert_eq!(word_backward(&b, p(0, 5)), p(0, 0));
        assert_eq!(word_backward(&b, p(0, 0)), p(0, 0));
    }

    #[test]
    fn word_end_stops_at_run_ends() {
        let b = Buffer::from_text("foo bar\nbaz");
        assert_eq!(word_end(&b, p(0, 0)), p(0, 2));
        assert_eq!(word_end(&b, p(0, 2)), p(0, 6));
        assert_eq!(word_end(&b, p(0, 6)), p(1, 2));
    }

    #[test]
    fn word_motions_skip_empty_lines() {
        let b = Buffer::from_text("foo\n\nbar");
        assert_eq!(word_forward(&b, p(0, 2)), p(2, 0));
        assert_eq!(word_backward(&b, p(2, 0)), p(0, 0));
    }

    #[test]
    fn inner_word_spans_the_word_under_cursor() {
        let b = Buffer::from_text("foo bar.baz");
        assert_eq!(word_object(&b, p(0, 5), false, false), (p(0, 4), p(0, 6)));
        assert_eq!(word_object(&b, p(0, 1), false, false), (p(0, 0), p(0, 2)));
        // A lone punctuation run is its own inner word.
        assert_eq!(word_object(&b, p(0, 7), false, false), (p(0, 7), p(0, 7)));
    }

    #[test]
    fn inner_big_word_spans_to_whitespace() {
        let b = Buffer::from_text("foo bar.baz qux");
        assert_eq!(word_object(&b, p(0, 5), false, true), (p(0, 4), p(0, 10)));
    }

    #[test]
    fn around_word_includes_trailing_space() {
        let b = Buffer::from_text("foo bar baz");
        assert_eq!(word_object(&b, p(0, 5), true, false), (p(0, 4), p(0, 7)));
    }

    #[test]
    fn around_word_at_line_end_includes_leading_space() {
        let b = Buffer::from_text("foo bar");
        assert_eq!(word_object(&b, p(0, 5), true, false), (p(0, 3), p(0, 6)));
    }

    #[test]
    fn inner_word_on_space_selects_the_space_run() {
        let b = Buffer::from_text("foo   bar");
        assert_eq!(word_object(&b, p(0, 4), false, false), (p(0, 3), p(0, 5)));
    }

    #[test]
    fn around_word_on_space_includes_following_word() {
        let b = Buffer::from_text("foo   bar.baz");
        // small aw stops at the punctuation boundary of the next word.
        assert_eq!(word_object(&b, p(0, 4), true, false), (p(0, 3), p(0, 8)));
    }

    #[test]
    fn word_object_on_empty_line_is_a_point() {
        let b = Buffer::from_text("foo\n\nbar");
        assert_eq!(word_object(&b, p(1, 0), false, false), (p(1, 0), p(1, 0)));
    }

    #[test]
    fn paragraph_motions_find_blank_lines() {
        let b = Buffer::from_text("a\nb\n\nc\nd\n\ne");
        assert_eq!(para_forward(&b, 0), 2);
        assert_eq!(para_forward(&b, 2), 5);
        assert_eq!(para_backward(&b, 6), 5);
        assert_eq!(para_backward(&b, 5), 2);
        assert_eq!(para_backward(&b, 1), 0);
    }
}
