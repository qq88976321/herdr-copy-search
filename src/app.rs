use crate::buffer::{Buffer, Pos};
use crate::lineedit::LineEdit;
use crate::motion;
use crate::patterns::{self, PatternDef};
use crate::search::{Direction, Search};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

pub enum Mode {
    Normal,
    Prompt {
        direction: Direction,
        input: LineEdit,
        origin: Pos,
        origin_top: usize,
    },
    PatternMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelKind {
    Char,
    Line,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    Copy(String),
}

pub struct App {
    pub buf: Buffer,
    pub cursor: Pos,
    pub top: usize,
    pub hoff: usize,
    pub view_rows: usize,
    pub view_cols: usize,
    pub search: Search,
    pub mode: Mode,
    pub sel: Option<(SelKind, Pos)>,
    pub message: Option<String>,
    pub quit: bool,
    want_col: usize,
    drag_origin: Option<Pos>,
    saved_pattern: String,
    pending_bottom: bool,
}

impl App {
    pub fn new(buf: Buffer, start_search: bool) -> Self {
        let cursor = Pos {
            row: buf.last_row(),
            col: 0,
        };
        let mode = if start_search {
            Mode::Prompt {
                direction: Direction::Forward,
                input: LineEdit::default(),
                origin: cursor,
                origin_top: 0,
            }
        } else {
            Mode::Normal
        };
        App {
            buf,
            cursor,
            top: 0,
            hoff: 0,
            view_rows: 1,
            view_cols: 80,
            search: Search::default(),
            mode,
            sel: None,
            message: None,
            quit: false,
            want_col: 0,
            drag_origin: None,
            saved_pattern: String::new(),
            pending_bottom: true,
        }
    }

    /// Called by the UI before each draw with the terminal size. The
    /// content owns every row; the bottom row is only overlaid while a
    /// prompt, menu, or message is showing (tmux-style seamless view).
    pub fn set_size(&mut self, cols: usize, rows: usize) {
        self.view_cols = cols.max(1);
        self.view_rows = rows.max(1);
        if self.pending_bottom {
            self.pending_bottom = false;
            self.top = self.buf.last_row().saturating_sub(self.view_rows - 1);
            if let Mode::Prompt { origin_top, .. } = &mut self.mode {
                *origin_top = self.top;
            }
        }
        self.ensure_visible();
    }

    /// While the bottom row is overlaid by the prompt or pattern menu,
    /// it does not count as visible content, so incremental search hits
    /// are never scrolled underneath the overlay.
    fn reserved_bottom(&self) -> usize {
        matches!(self.mode, Mode::Prompt { .. } | Mode::PatternMenu) as usize
    }

    fn content_rows(&self) -> usize {
        (self.view_rows - self.reserved_bottom()).max(1)
    }

    fn max_top(&self) -> usize {
        self.buf
            .last_row()
            .saturating_sub(self.content_rows().saturating_sub(1))
    }

    fn ensure_visible(&mut self) {
        let rows = self.content_rows();
        if self.cursor.row < self.top {
            self.top = self.cursor.row;
        }
        if self.cursor.row >= self.top + rows {
            self.top = self.cursor.row + 1 - rows;
        }
        if self.top > self.max_top() {
            self.top = self.max_top();
        }
        self.ensure_visible_h();
    }

    /// Horizontal follow in display cells: the cursor's whole cell range
    /// must stay inside [hoff, hoff + view_cols).
    fn ensure_visible_h(&mut self) {
        let line = self.buf.line(self.cursor.row);
        let x = line.width_before(self.cursor.col);
        let cw = line.cell_width(self.cursor.col);
        if x < self.hoff {
            self.hoff = x;
        }
        if x + cw > self.hoff + self.view_cols {
            self.hoff = x + cw - self.view_cols;
        }
    }

    /// Wheel scrolling moves the view and drags the cursor along.
    fn scroll(&mut self, delta: isize) {
        let top = self.top as isize + delta;
        self.top = top.clamp(0, self.max_top() as isize) as usize;
        let low = self.top;
        let high = self.top + self.view_rows - 1;
        let row = self.cursor.row.clamp(low, high.min(self.buf.last_row()));
        self.cursor = Pos {
            row,
            col: self.want_col.min(self.buf.max_col(row)),
        };
        self.ensure_visible_h();
    }

    fn move_vertical(&mut self, delta: isize) {
        let row =
            (self.cursor.row as isize + delta).clamp(0, self.buf.last_row() as isize) as usize;
        self.cursor = Pos {
            row,
            col: self.want_col.min(self.buf.max_col(row)),
        };
        self.ensure_visible();
    }

    fn goto(&mut self, p: Pos) {
        self.cursor = self.buf.clamp(p);
        self.want_col = self.cursor.col;
        self.ensure_visible();
    }

    pub fn selection_text(&self) -> Option<String> {
        let (kind, anchor) = self.sel?;
        Some(match kind {
            SelKind::Char => self.buf.slice_inclusive(anchor, self.cursor),
            SelKind::Line => self.buf.slice_lines(anchor.row, self.cursor.row),
        })
    }

    /// Inclusive selected col range for a row, if the selection covers it.
    pub fn sel_range(&self, row: usize) -> Option<(usize, usize)> {
        let (kind, anchor) = self.sel?;
        let (a, b) = if (anchor.row, anchor.col) <= (self.cursor.row, self.cursor.col) {
            (anchor, self.cursor)
        } else {
            (self.cursor, anchor)
        };
        if row < a.row || row > b.row {
            return None;
        }
        match kind {
            SelKind::Line => Some((0, usize::MAX)),
            SelKind::Char => {
                let s = if row == a.row { a.col } else { 0 };
                let e = if row == b.row { b.col } else { usize::MAX };
                Some((s, e))
            }
        }
    }

    fn yank(&mut self, exit: bool) -> Option<Effect> {
        let text = self.selection_text().or_else(|| {
            self.search.match_at(self.cursor).map(|m| {
                self.buf.slice_inclusive(
                    Pos {
                        row: m.row,
                        col: m.start,
                    },
                    Pos {
                        row: m.row,
                        col: m.end - 1,
                    },
                )
            })
        })?;
        if exit {
            self.quit = true;
        } else {
            self.message = Some(format!("copied {} chars", text.chars().count()));
            self.sel = None;
        }
        Some(Effect::Copy(text))
    }

    fn jump_match(&mut self, forward: bool) {
        if self.search.pattern.is_empty() {
            self.message = Some("no previous search".into());
            return;
        }
        let m = if forward {
            self.search.next(self.cursor)
        } else {
            self.search.prev(self.cursor)
        };
        match m {
            Some(m) => self.goto(Pos {
                row: m.row,
                col: m.start,
            }),
            None => self.message = Some(format!("no match: {}", self.search.pattern)),
        }
    }

    fn open_prompt(&mut self, direction: Direction) {
        self.saved_pattern = self.search.pattern.clone();
        self.mode = Mode::Prompt {
            direction,
            input: LineEdit::default(),
            origin: self.cursor,
            origin_top: self.top,
        };
    }

    fn incremental(&mut self, input: &str, dir: Direction, origin: Pos, origin_top: usize) {
        self.search.run(&self.buf, input, dir);
        if input.is_empty() {
            self.cursor = origin;
            self.top = origin_top;
            return;
        }
        match self.search.seek(origin, dir, true) {
            Some(m) => self.goto(Pos {
                row: m.row,
                col: m.start,
            }),
            None => {
                self.cursor = origin;
                self.top = origin_top;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Effect> {
        self.message = None;
        match self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Prompt { .. } => {
                self.handle_prompt(key);
                None
            }
            Mode::PatternMenu => {
                self.handle_pattern_menu(key);
                None
            }
        }
    }

    /// Predefined pattern search, copycat style: backward from the
    /// cursor so `n` walks up through older output and `N` back down.
    fn activate_pattern(&mut self, def: &PatternDef) {
        self.search.run(&self.buf, def.regex, Direction::Backward);
        let n = self.search.matches.len();
        if n == 0 {
            self.message = Some(format!("no match: {}", def.name));
            return;
        }
        if let Some(m) = self.search.seek(self.cursor, Direction::Backward, true) {
            self.goto(Pos {
                row: m.row,
                col: m.start,
            });
        }
        self.message = Some(format!("{}: {} matches", def.name, n));
    }

    fn handle_pattern_menu(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char(c) => {
                self.mode = Mode::Normal;
                match patterns::by_key(c) {
                    Some(def) => self.activate_pattern(def),
                    None => self.message = Some(format!("no pattern: {c}")),
                }
            }
            _ => {}
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Option<Effect> {
        use KeyCode::*;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let half = (self.view_rows / 2).max(1) as isize;
        let page = self.view_rows.max(1) as isize;
        if key.modifiers.contains(KeyModifiers::ALT) {
            if let Char(c) = key.code {
                if let Some(def) = patterns::by_key(c) {
                    self.activate_pattern(def);
                }
                return None;
            }
        }
        match key.code {
            Char('p') if !ctrl => self.mode = Mode::PatternMenu,
            Char('q') if !ctrl => self.quit = true,
            Esc => {
                if self.sel.is_some() {
                    self.sel = None;
                } else {
                    self.quit = true;
                }
            }
            Char('v') if !ctrl => {
                self.sel = match self.sel {
                    Some((SelKind::Char, _)) => None,
                    _ => Some((SelKind::Char, self.cursor)),
                };
            }
            Char('V') => {
                self.sel = match self.sel {
                    Some((SelKind::Line, _)) => None,
                    _ => Some((SelKind::Line, self.cursor)),
                };
            }
            Char('y') if !ctrl => return self.yank(false),
            Enter => return self.yank(true),
            Char('/') => self.open_prompt(Direction::Forward),
            Char('?') => self.open_prompt(Direction::Backward),
            Char('n') => self.jump_match(true),
            Char('N') => self.jump_match(false),
            Char('h') | Left => {
                let col = self.cursor.col.saturating_sub(1);
                self.goto(Pos {
                    row: self.cursor.row,
                    col,
                });
            }
            Char('l') | Right => {
                self.goto(Pos {
                    row: self.cursor.row,
                    col: self.cursor.col + 1,
                });
            }
            Char('j') | Down => self.move_vertical(1),
            Char('k') | Up => self.move_vertical(-1),
            Char('0') | Home => self.goto(Pos {
                row: self.cursor.row,
                col: 0,
            }),
            Char('^') => {
                let col = self.buf.first_non_blank(self.cursor.row);
                self.goto(Pos {
                    row: self.cursor.row,
                    col,
                });
            }
            Char('$') | End => {
                self.goto(Pos {
                    row: self.cursor.row,
                    col: self.buf.max_col(self.cursor.row),
                });
                self.want_col = usize::MAX;
            }
            Char('w') if !ctrl => {
                let p = motion::word_forward(&self.buf, self.cursor);
                self.goto(p);
            }
            Char('b') if !ctrl => {
                let p = motion::word_backward(&self.buf, self.cursor);
                self.goto(p);
            }
            Char('e') if !ctrl => {
                let p = motion::word_end(&self.buf, self.cursor);
                self.goto(p);
            }
            Char('{') => {
                let row = motion::para_backward(&self.buf, self.cursor.row);
                self.goto(Pos { row, col: 0 });
            }
            Char('}') => {
                let row = motion::para_forward(&self.buf, self.cursor.row);
                self.goto(Pos { row, col: 0 });
            }
            Char('g') if !ctrl => self.goto(Pos { row: 0, col: 0 }),
            Char('G') => self.goto(Pos {
                row: self.buf.last_row(),
                col: 0,
            }),
            Char('f') if ctrl => self.move_vertical(page),
            Char('b') if ctrl => self.move_vertical(-page),
            Char('d') if ctrl => self.move_vertical(half),
            Char('u') if ctrl => self.move_vertical(-half),
            PageDown => self.move_vertical(page),
            PageUp => self.move_vertical(-page),
            _ => {}
        }
        None
    }

    fn handle_prompt(&mut self, key: KeyEvent) {
        use KeyCode::*;
        let Mode::Prompt {
            direction,
            mut input,
            origin,
            origin_top,
        } = std::mem::replace(&mut self.mode, Mode::Normal)
        else {
            return;
        };
        match key.code {
            Esc => {
                // Cancel: restore view and the previous search highlights.
                self.cursor = origin;
                self.top = origin_top;
                let saved = self.saved_pattern.clone();
                let dir = self.search.direction.unwrap_or(direction);
                self.search.run(&self.buf, &saved, dir);
                return;
            }
            Enter => {
                if input.is_empty() {
                    // Repeat the previous search in this direction.
                    let saved = self.saved_pattern.clone();
                    if saved.is_empty() {
                        return;
                    }
                    self.search.run(&self.buf, &saved, direction);
                    match self.search.seek(origin, direction, false) {
                        Some(m) => self.goto(Pos {
                            row: m.row,
                            col: m.start,
                        }),
                        None => self.message = Some(format!("no match: {}", saved)),
                    }
                } else if self.search.matches.is_empty() {
                    self.message = Some(format!("no match: {}", input.text()));
                }
                return;
            }
            Backspace if input.is_empty() => {
                // Backspace on an empty prompt closes it, tmux-style.
                self.cursor = origin;
                self.top = origin_top;
                let saved = self.saved_pattern.clone();
                let dir = self.search.direction.unwrap_or(direction);
                self.search.run(&self.buf, &saved, dir);
                return;
            }
            _ => {
                if !input.handle_key(key) {
                    self.mode = Mode::Prompt {
                        direction,
                        input,
                        origin,
                        origin_top,
                    };
                    return;
                }
            }
        }
        // Restore the prompt before jumping so the reserved bottom row
        // is accounted for while scrolling to the incremental hit.
        let query = input.text();
        self.mode = Mode::Prompt {
            direction,
            input,
            origin,
            origin_top,
        };
        self.incremental(&query, direction, origin, origin_top);
    }

    pub fn handle_mouse(&mut self, ev: MouseEvent) {
        match ev.kind {
            MouseEventKind::ScrollUp => self.scroll(-3),
            MouseEventKind::ScrollDown => self.scroll(3),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(p) = self.pos_at(ev.column, ev.row) {
                    self.sel = None;
                    self.cursor = p;
                    self.want_col = p.col;
                    self.drag_origin = Some(p);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(origin) = self.drag_origin {
                    if self.sel.is_none() {
                        self.sel = Some((SelKind::Char, origin));
                    }
                    if let Some(p) = self.pos_at(ev.column, ev.row) {
                        self.cursor = p;
                        self.want_col = p.col;
                        self.ensure_visible();
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_origin = None;
            }
            _ => {}
        }
    }

    fn pos_at(&self, x: u16, y: u16) -> Option<Pos> {
        let y = y as usize;
        if y >= self.view_rows {
            return None;
        }
        let row = (self.top + y).min(self.buf.last_row());
        let col = self.buf.line(row).col_at_x(self.hoff + x as usize);
        Some(Pos { row, col })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn app(text: &str) -> App {
        let mut a = App::new(Buffer::from_text(text), false);
        a.set_size(80, 11); // 10 content rows + status
        a
    }

    #[test]
    fn starts_at_bottom() {
        let a = app("one\ntwo\nthree");
        assert_eq!(a.cursor, Pos { row: 2, col: 0 });
    }

    #[test]
    fn incremental_search_jumps_and_esc_restores() {
        let mut a = app("alpha\nbeta\ngamma");
        a.handle_key(key('/'));
        a.handle_key(key('b'));
        a.handle_key(key('e'));
        assert_eq!(a.cursor, Pos { row: 1, col: 0 });
        a.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(a.cursor, Pos { row: 2, col: 0 });
        assert!(matches!(a.mode, Mode::Normal));
    }

    #[test]
    fn commit_search_then_n_wraps() {
        let mut a = app("foo\nbar\nfoo\nbaz");
        a.handle_key(key('/'));
        a.handle_key(key('f'));
        a.handle_key(key('o'));
        a.handle_key(key('o'));
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        // Search opened at the bottom row wraps to the first match.
        assert_eq!(a.cursor.row, 0);
        a.handle_key(key('n'));
        assert_eq!(a.cursor.row, 2);
        a.handle_key(key('n'));
        assert_eq!(a.cursor.row, 0);
    }

    #[test]
    fn charwise_selection_yank() {
        let mut a = app("hello world");
        a.handle_key(key('g'));
        a.handle_key(key('v'));
        for _ in 0..4 {
            a.handle_key(key('l'));
        }
        let eff = a.handle_key(key('y'));
        assert_eq!(eff, Some(Effect::Copy("hello".into())));
        assert!(a.sel.is_none());
        assert!(!a.quit);
    }

    #[test]
    fn linewise_selection_enter_yanks_and_quits() {
        let mut a = app("one\ntwo\nthree");
        a.handle_key(key('g'));
        a.handle_key(key('V'));
        a.handle_key(key('j'));
        let eff = a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(eff, Some(Effect::Copy("one\ntwo\n".into())));
        assert!(a.quit);
    }

    #[test]
    fn yank_without_selection_copies_match_under_cursor() {
        let mut a = app("foo bar");
        a.handle_key(key('/'));
        a.handle_key(key('b'));
        a.handle_key(key('a'));
        a.handle_key(key('r'));
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let eff = a.handle_key(key('y'));
        assert_eq!(eff, Some(Effect::Copy("bar".into())));
    }

    #[test]
    fn question_mark_searches_backward() {
        let mut a = app("foo\nmid\nfoo\nend");
        a.handle_key(key('G'));
        a.handle_key(key('?'));
        a.handle_key(key('f'));
        a.handle_key(key('o'));
        a.handle_key(key('o'));
        assert_eq!(a.cursor.row, 2);
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        a.handle_key(key('n'));
        assert_eq!(a.cursor.row, 0);
    }

    #[test]
    fn prompt_left_arrow_edits_mid_query() {
        let mut a = app("alpha\nbeta\ngamma");
        a.handle_key(key('/'));
        for c in "bta".chars() {
            a.handle_key(key(c));
        }
        // No match yet: the view stays at the origin.
        assert_eq!(a.cursor.row, 2);
        let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        a.handle_key(left);
        a.handle_key(left);
        a.handle_key(key('e'));
        assert_eq!(a.cursor, Pos { row: 1, col: 0 });
        let Mode::Prompt { input, .. } = &a.mode else {
            panic!("prompt should stay open");
        };
        assert_eq!(input.text(), "beta");
    }

    #[test]
    fn prompt_home_and_delete_edit_the_query() {
        let mut a = app("xfoo\nfoo");
        a.handle_key(key('/'));
        for c in "xfoo".chars() {
            a.handle_key(key(c));
        }
        a.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        a.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(a.search.pattern, "foo");
        assert_eq!(a.search.matches.len(), 2);
    }

    fn alt(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
    }

    #[test]
    fn pattern_menu_selects_url_search() {
        let mut a = app("visit https://x.dev now\nplain tail");
        a.handle_key(key('p'));
        assert!(matches!(a.mode, Mode::PatternMenu));
        a.handle_key(key('u'));
        assert!(matches!(a.mode, Mode::Normal));
        assert_eq!(a.cursor, Pos { row: 0, col: 6 });
        assert_eq!(a.search.matches.len(), 1);
    }

    #[test]
    fn pattern_menu_esc_cancels_and_unknown_key_reports() {
        let mut a = app("text");
        a.handle_key(key('p'));
        a.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(a.mode, Mode::Normal));
        assert!(!a.quit);
        a.handle_key(key('p'));
        a.handle_key(key('z'));
        assert_eq!(a.message.as_deref(), Some("no pattern: z"));
    }

    #[test]
    fn alt_key_activates_pattern_backward_and_n_walks_up() {
        let mut a = app("num 1\nnum 2\nnum 3");
        a.handle_key(alt('d'));
        // Nearest match at or above the cursor (bottom row, col 0).
        assert_eq!(a.cursor, Pos { row: 1, col: 4 });
        a.handle_key(key('n'));
        assert_eq!(a.cursor, Pos { row: 0, col: 4 });
        a.handle_key(key('N'));
        assert_eq!(a.cursor, Pos { row: 1, col: 4 });
    }

    #[test]
    fn view_uses_all_rows() {
        let text = (0..30).map(|i| format!("l{i}\n")).collect::<String>();
        let mut a = App::new(Buffer::from_text(&text), false);
        a.set_size(80, 10);
        assert_eq!(a.view_rows, 10);
        assert_eq!(a.top, 20, "bottom row is content, not chrome");
    }

    #[test]
    fn prompt_reserves_bottom_row_for_matches() {
        let text = (0..30).map(|i| format!("l{i}\n")).collect::<String>();
        let mut a = App::new(Buffer::from_text(&text), false);
        a.set_size(80, 10);
        a.handle_key(key('/'));
        for c in "l29".chars() {
            a.handle_key(key(c));
        }
        // The hit on the last row scrolls one line up so it is not
        // hidden under the prompt overlay.
        assert_eq!(a.cursor.row, 29);
        assert_eq!(a.top, 21);
    }

    #[test]
    fn horizontal_scroll_follows_cursor() {
        let long = "x".repeat(100);
        let mut a = app(&long);
        a.handle_key(key('$'));
        // Cursor cell 99 must be inside [hoff, hoff + 80).
        assert_eq!(a.hoff, 20);
        a.handle_key(key('0'));
        assert_eq!(a.hoff, 0);
    }

    #[test]
    fn horizontal_scroll_keeps_wide_cursor_fully_visible() {
        let wide = "\u{4e2d}".repeat(50);
        let mut a = app(&wide);
        a.handle_key(key('$'));
        // The cursor char occupies display cells [98, 100).
        assert_eq!(a.hoff, 20);
    }

    #[test]
    fn want_col_is_kept_across_short_lines() {
        let mut a = app("longline\nab\nlongline");
        a.handle_key(key('g'));
        a.handle_key(key('$'));
        a.handle_key(key('j'));
        assert_eq!(a.cursor.col, 1);
        a.handle_key(key('j'));
        assert_eq!(a.cursor.col, 7);
    }
}
