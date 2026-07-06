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
    fn paragraph_motions_find_blank_lines() {
        let b = Buffer::from_text("a\nb\n\nc\nd\n\ne");
        assert_eq!(para_forward(&b, 0), 2);
        assert_eq!(para_forward(&b, 2), 5);
        assert_eq!(para_backward(&b, 6), 5);
        assert_eq!(para_backward(&b, 5), 2);
        assert_eq!(para_backward(&b, 1), 0);
    }
}
