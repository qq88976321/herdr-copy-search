use crate::buffer::{Buffer, Pos};

/// One search hit. `start`/`end` are char indices into the row (end
/// exclusive), matching the Buffer coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub row: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    pub fn flipped(self) -> Self {
        match self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}

pub struct Search {
    pub pattern: String,
    pub direction: Option<Direction>,
    pub matches: Vec<Match>,
    pub current: Option<usize>,
    /// Smartcase (vi-style, default on): a pattern with no uppercase is
    /// matched case-insensitively; any uppercase makes it case-sensitive.
    /// Toggled off, search is always case-sensitive.
    pub smartcase: bool,
}

impl Default for Search {
    fn default() -> Self {
        Search {
            pattern: String::new(),
            direction: None,
            matches: Vec::new(),
            current: None,
            smartcase: true,
        }
    }
}

impl Search {
    /// Recompute all matches for a pattern. The pattern is tried as a
    /// regex first (tmux search semantics) and falls back to a literal
    /// match when it is not a valid regex.
    pub fn run(&mut self, buf: &Buffer, pattern: &str, direction: Direction) {
        self.pattern = pattern.to_string();
        self.direction = Some(direction);
        self.matches.clear();
        self.current = None;
        if pattern.is_empty() {
            return;
        }
        // Smartcase: fold case only when the pattern has no uppercase.
        let ci = self.smartcase && !pattern.chars().any(char::is_uppercase);
        let build = |pat: &str| {
            regex::RegexBuilder::new(pat)
                .case_insensitive(ci)
                .build()
                .ok()
        };
        let re = match build(pattern).or_else(|| build(&regex::escape(pattern))) {
            Some(re) => re,
            None => return,
        };
        for (row, line) in buf.lines.iter().enumerate() {
            let s = line.text();
            for m in re.find_iter(&s) {
                if m.start() == m.end() {
                    continue;
                }
                let start = s[..m.start()].chars().count();
                let end = start + s[m.start()..m.end()].chars().count();
                self.matches.push(Match { row, start, end });
            }
        }
    }

    pub fn clear(&mut self) {
        self.pattern.clear();
        self.matches.clear();
        self.current = None;
    }

    /// Find the nearest match from `pos` in `dir`. With `inclusive`, a
    /// match starting exactly at `pos` counts (used while typing the
    /// pattern); without, the cursor position is skipped (n/N).
    pub fn seek(&mut self, pos: Pos, dir: Direction, inclusive: bool) -> Option<Match> {
        if self.matches.is_empty() {
            self.current = None;
            return None;
        }
        let idx = match dir {
            Direction::Forward => {
                let key = if inclusive {
                    (pos.row, pos.col)
                } else {
                    (pos.row, pos.col + 1)
                };
                self.matches
                    .iter()
                    .position(|m| (m.row, m.start) >= key)
                    .unwrap_or(0)
            }
            Direction::Backward => {
                let key = (pos.row, pos.col);
                let hit = if inclusive {
                    self.matches.iter().rposition(|m| (m.row, m.start) <= key)
                } else {
                    self.matches.iter().rposition(|m| (m.row, m.start) < key)
                };
                hit.unwrap_or(self.matches.len() - 1)
            }
        };
        self.current = Some(idx);
        Some(self.matches[idx])
    }

    /// n: continue in the search direction.
    pub fn next(&mut self, pos: Pos) -> Option<Match> {
        let dir = self.direction?;
        self.seek(pos, dir, false)
    }

    /// N: continue against the search direction.
    pub fn prev(&mut self, pos: Pos) -> Option<Match> {
        let dir = self.direction?.flipped();
        self.seek(pos, dir, false)
    }

    /// The match covering `pos`, if any.
    pub fn match_at(&self, pos: Pos) -> Option<Match> {
        self.matches
            .iter()
            .copied()
            .find(|m| m.row == pos.row && pos.col >= m.start && pos.col < m.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf() -> Buffer {
        Buffer::from_text("foo bar foo\nbaz\nfoo end")
    }

    #[test]
    fn finds_all_matches_with_char_indices() {
        let b = buf();
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Forward);
        assert_eq!(
            s.matches,
            vec![
                Match {
                    row: 0,
                    start: 0,
                    end: 3
                },
                Match {
                    row: 0,
                    start: 8,
                    end: 11
                },
                Match {
                    row: 2,
                    start: 0,
                    end: 3
                },
            ]
        );
    }

    #[test]
    fn smartcase_lowercase_matches_any_case() {
        let b = Buffer::from_text("Foo foo FOO");
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Forward);
        assert_eq!(s.matches.len(), 3);
    }

    #[test]
    fn smartcase_uppercase_is_case_sensitive() {
        let b = Buffer::from_text("Foo foo FOO");
        let mut s = Search::default();
        s.run(&b, "Foo", Direction::Forward);
        assert_eq!(
            s.matches,
            vec![Match {
                row: 0,
                start: 0,
                end: 3
            }]
        );
    }

    #[test]
    fn smartcase_off_is_always_case_sensitive() {
        let b = Buffer::from_text("Foo foo FOO");
        let mut s = Search {
            smartcase: false,
            ..Default::default()
        };
        s.run(&b, "foo", Direction::Forward);
        assert_eq!(
            s.matches,
            vec![Match {
                row: 0,
                start: 4,
                end: 7
            }]
        );
    }

    #[test]
    fn invalid_regex_falls_back_to_literal() {
        let b = Buffer::from_text("a(b\nc");
        let mut s = Search::default();
        s.run(&b, "a(b", Direction::Forward);
        assert_eq!(s.matches.len(), 1);
    }

    #[test]
    fn wide_chars_use_char_columns() {
        let b = Buffer::from_text("\u{4e2d}\u{6587}foo");
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Forward);
        assert_eq!(
            s.matches,
            vec![Match {
                row: 0,
                start: 2,
                end: 5
            }]
        );
    }

    #[test]
    fn seek_forward_wraps() {
        let b = buf();
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Forward);
        let m = s
            .seek(Pos { row: 2, col: 1 }, Direction::Forward, false)
            .unwrap();
        assert_eq!(m.row, 0);
        assert_eq!(m.start, 0);
    }

    #[test]
    fn seek_inclusive_stays_on_cursor_match() {
        let b = buf();
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Forward);
        let m = s
            .seek(Pos { row: 0, col: 0 }, Direction::Forward, true)
            .unwrap();
        assert_eq!((m.row, m.start), (0, 0));
        let m = s
            .seek(Pos { row: 0, col: 0 }, Direction::Forward, false)
            .unwrap();
        assert_eq!((m.row, m.start), (0, 8));
    }

    #[test]
    fn seek_backward_wraps() {
        let b = buf();
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Backward);
        let m = s
            .seek(Pos { row: 0, col: 0 }, Direction::Backward, false)
            .unwrap();
        assert_eq!(m.row, 2);
    }

    #[test]
    fn next_and_prev_respect_direction() {
        let b = buf();
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Backward);
        // n with a backward search keeps going backward.
        let m = s.next(Pos { row: 2, col: 0 }).unwrap();
        assert_eq!((m.row, m.start), (0, 8));
        let m = s.prev(Pos { row: 0, col: 8 }).unwrap();
        assert_eq!((m.row, m.start), (2, 0));
    }

    #[test]
    fn match_at_covers_position() {
        let b = buf();
        let mut s = Search::default();
        s.run(&b, "foo", Direction::Forward);
        assert!(s.match_at(Pos { row: 0, col: 9 }).is_some());
        assert!(s.match_at(Pos { row: 0, col: 3 }).is_none());
    }
}
