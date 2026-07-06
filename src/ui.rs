use crate::app::{App, Mode, SelKind};
use crate::buffer::Pos;
use crate::search::Direction;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{
    Attribute, Color, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use std::io::{self, Write};
use unicode_width::UnicodeWidthChar;

/// Highlight class of a rendered cell. Precedence when several apply:
/// Cursor > Sel > CurrentMatch > Match > None.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hl {
    None,
    Match,
    CurrentMatch,
    Sel,
    Cursor,
}

fn hl_at(app: &App, row: usize, col: usize, sel: Option<(usize, usize)>) -> Hl {
    if row == app.cursor.row && col == app.cursor.col {
        return Hl::Cursor;
    }
    if let Some((s, e)) = sel {
        if col >= s && col <= e {
            return Hl::Sel;
        }
    }
    if let Some(i) = app.search.current {
        let m = app.search.matches[i];
        if m.row == row && col >= m.start && col < m.end {
            return Hl::CurrentMatch;
        }
    }
    if app.search.match_at(Pos { row, col }).is_some() {
        return Hl::Match;
    }
    Hl::None
}

fn push_char(spans: &mut Vec<(Hl, String)>, hl: Hl, c: char) {
    match spans.last_mut() {
        Some((h, s)) if *h == hl => s.push(c),
        _ => spans.push((hl, c.to_string())),
    }
}

/// Per-row spans in char space, merging selection, search matches, the
/// current match, and the cursor cell. The cursor gets a virtual space
/// cell when it sits past the end of the line (empty lines).
pub fn row_spans(app: &App, row: usize) -> Vec<(Hl, String)> {
    let line = app.buf.line(row);
    let sel = app.sel_range(row);
    let mut spans = Vec::new();
    for (col, &c) in line.chars.iter().enumerate() {
        push_char(&mut spans, hl_at(app, row, col, sel), c);
    }
    if row == app.cursor.row && app.cursor.col >= line.len() {
        spans.push((Hl::Cursor, " ".into()));
    }
    spans
}

/// Clip spans to the viewport [hoff, hoff + view_cols) in display cells.
/// A wide char straddling the left edge renders as a space; a char that
/// would cross the right edge is dropped.
pub fn clip_spans(spans: Vec<(Hl, String)>, hoff: usize, view_cols: usize) -> Vec<(Hl, String)> {
    let end = hoff + view_cols;
    let mut out = Vec::new();
    let mut x = 0usize;
    for (hl, text) in spans {
        for c in text.chars() {
            let w = c.width().unwrap_or(0).max(1);
            if x + w <= hoff {
                x += w;
                continue;
            }
            if x >= end || x + w > end {
                return out;
            }
            if x < hoff {
                push_char(&mut out, hl, ' ');
            } else {
                push_char(&mut out, hl, c);
            }
            x += w;
        }
    }
    out
}

/// Content of the bottom line: prompt while searching, else a message
/// override, else the status summary.
pub fn bottom_line(app: &App) -> String {
    if let Mode::Prompt {
        direction, input, ..
    } = &app.mode
    {
        let c = match direction {
            Direction::Forward => '/',
            Direction::Backward => '?',
        };
        return format!("{c}{input}");
    }
    if matches!(app.mode, Mode::PatternMenu) {
        return crate::patterns::menu_line();
    }
    if let Some(msg) = &app.message {
        return msg.clone();
    }
    let mode = match app.sel {
        Some((SelKind::Char, _)) => "-- VISUAL --",
        Some((SelKind::Line, _)) => "-- VISUAL LINE --",
        None => "-- COPY --",
    };
    let mut out = mode.to_string();
    if !app.search.pattern.is_empty() {
        out.push_str(&format!("  /{}", app.search.pattern));
        match app.search.current {
            Some(i) => out.push_str(&format!("  [{}/{}]", i + 1, app.search.matches.len())),
            None => out.push_str(&format!("  [{}]", app.search.matches.len())),
        }
    }
    let pct = (app.cursor.row + 1) * 100 / (app.buf.last_row() + 1);
    out.push_str(&format!("  {pct}%"));
    out
}

fn style(out: &mut impl Write, hl: Hl) -> io::Result<()> {
    match hl {
        Hl::None => {}
        Hl::Match => queue!(
            out,
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::DarkYellow)
        )?,
        Hl::CurrentMatch => queue!(
            out,
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::Yellow)
        )?,
        Hl::Sel => queue!(out, SetAttribute(Attribute::Reverse))?,
        Hl::Cursor => queue!(
            out,
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::Cyan)
        )?,
    }
    Ok(())
}

/// Draw the whole screen: content rows then the bottom line.
pub fn draw(out: &mut impl Write, app: &App) -> io::Result<()> {
    for y in 0..app.view_rows {
        queue!(out, MoveTo(0, y as u16), Clear(ClearType::CurrentLine))?;
        let row = app.top + y;
        if row <= app.buf.last_row() {
            for (hl, text) in clip_spans(row_spans(app, row), app.hoff, app.view_cols) {
                style(out, hl)?;
                queue!(out, crossterm::style::Print(text), ResetColor)?;
                queue!(out, SetAttribute(Attribute::Reset))?;
            }
        }
    }
    queue!(
        out,
        MoveTo(0, app.view_rows as u16),
        Clear(ClearType::CurrentLine)
    )?;
    let mut text = bottom_line(app);
    let mut w = 0usize;
    text.retain(|c| {
        w += c.width().unwrap_or(0).max(1);
        w < app.view_cols
    });
    queue!(out, crossterm::style::Print(&text))?;
    if matches!(app.mode, Mode::Prompt { .. }) {
        queue!(
            out,
            SetAttribute(Attribute::Reverse),
            crossterm::style::Print(" "),
            SetAttribute(Attribute::Reset)
        )?;
    }
    out.flush()
}

/// RAII terminal state: raw mode, alternate screen, mouse capture, and
/// a panic hook that restores the terminal before printing the panic.
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, Hide)?;
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = TerminalGuard::restore();
            hook(info);
        }));
        Ok(TerminalGuard)
    }

    fn restore() -> io::Result<()> {
        execute!(
            io::stdout(),
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = Self::restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::buffer::Buffer;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn app(text: &str) -> App {
        let mut a = App::new(Buffer::from_text(text), false);
        a.set_size(80, 11);
        a
    }

    fn searched(text: &str, pat: &str) -> App {
        let mut a = app(text);
        a.handle_key(key('/'));
        for c in pat.chars() {
            a.handle_key(key(c));
        }
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        a
    }

    #[test]
    fn spans_mark_current_match_and_other_matches() {
        // Search opened from the bottom wraps to row 0 as current.
        let a = searched("foo bar foo", "foo");
        let spans = row_spans(&a, 0);
        assert_eq!(
            spans,
            vec![
                (Hl::Cursor, "f".into()),
                (Hl::CurrentMatch, "oo".into()),
                (Hl::None, " bar ".into()),
                (Hl::Match, "foo".into()),
            ]
        );
    }

    #[test]
    fn selection_wins_over_match_but_not_cursor() {
        let mut a = searched("foo bar foo", "bar");
        a.handle_key(key('v'));
        for _ in 0..2 {
            a.handle_key(key('l'));
        }
        // Selection covers cols 4..=6 ("bar"), cursor at col 6.
        let spans = row_spans(&a, 0);
        assert_eq!(
            spans,
            vec![
                (Hl::None, "foo ".into()),
                (Hl::Sel, "ba".into()),
                (Hl::Cursor, "r".into()),
                (Hl::None, " foo".into()),
            ]
        );
    }

    #[test]
    fn cursor_on_empty_line_gets_virtual_cell() {
        let mut a = app("foo\n\nbar");
        a.handle_key(key('g'));
        a.handle_key(key('j'));
        let spans = row_spans(&a, 1);
        assert_eq!(spans, vec![(Hl::Cursor, " ".into())]);
    }

    #[test]
    fn clip_drops_cells_left_of_hoff_and_pads_straddling_wide_char() {
        let spans = vec![(Hl::None, "\u{4e2d}\u{6587}abc".into())];
        // hoff=1 cuts through the first wide char.
        assert_eq!(
            clip_spans(spans, 1, 80),
            vec![(Hl::None, " \u{6587}abc".into())]
        );
    }

    #[test]
    fn clip_stops_at_right_edge_without_splitting_wide_char() {
        let spans = vec![(Hl::None, "ab\u{4e2d}cd".into())];
        // Viewport of 3 cells: 'a', 'b' fit; the wide char would span
        // cells 2..4 and is dropped along with everything after it.
        assert_eq!(clip_spans(spans, 0, 3), vec![(Hl::None, "ab".into())]);
    }

    #[test]
    fn bottom_line_shows_prompt_then_status() {
        let mut a = app("foo bar");
        a.handle_key(key('/'));
        a.handle_key(key('b'));
        assert_eq!(bottom_line(&a), "/b");
        a.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(bottom_line(&a), "-- COPY --  /b  [1/1]  100%");
    }

    #[test]
    fn bottom_line_message_overrides_status() {
        let mut a = app("foo");
        a.handle_key(key('n'));
        assert_eq!(bottom_line(&a), "no previous search");
    }
}
