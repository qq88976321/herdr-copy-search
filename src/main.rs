use crossterm::event::{self, Event, KeyEventKind};
use herdr_copy_search::app::{App, Effect};
use herdr_copy_search::buffer::Buffer;
use herdr_copy_search::extract::{ExtractApp, ExtractEffect};
use herdr_copy_search::herdr::{self, Herdr};
use herdr_copy_search::{osc52, ui};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str =
    "usage: herdr-copy-search [--mode copy|search|extract] [--pane ID] [--lines N] [--input FILE]";

/// The herdr server caps pane reads at ~1000 lines (spike-verified),
/// so asking for more only future-proofs against a raised cap.
const DEFAULT_LINES: usize = 1000;

#[derive(Debug, PartialEq, Eq)]
enum RunMode {
    Copy,
    Search,
    Extract,
}

#[derive(Debug, PartialEq, Eq)]
struct Cli {
    mode: RunMode,
    input: Option<PathBuf>,
    pane: Option<String>,
    lines: usize,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Cli, String> {
    let mut cli = Cli {
        mode: RunMode::Copy,
        input: None,
        pane: None,
        lines: DEFAULT_LINES,
    };
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--mode" => {
                cli.mode = match value("--mode")?.as_str() {
                    "copy" => RunMode::Copy,
                    "search" => RunMode::Search,
                    "extract" => RunMode::Extract,
                    other => return Err(format!("unknown mode: {other}")),
                }
            }
            "--input" => cli.input = Some(PathBuf::from(value("--input")?)),
            "--pane" => cli.pane = Some(value("--pane")?),
            "--lines" => {
                cli.lines = value("--lines")?
                    .parse()
                    .map_err(|_| "--lines needs a number".to_string())?
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(cli)
}

/// Input order: explicit file, then a herdr source pane, then piped
/// stdin (dev use; crossterm reads events from /dev/tty instead).
/// The herdr path reads twice: logical text for search/yank/tokens and
/// styled wrapped rows for display, composed by Buffer::from_reads.
fn acquire_buffer(cli: &Cli, source_pane: Option<&str>) -> io::Result<Buffer> {
    if let Some(path) = &cli.input {
        return Ok(Buffer::from_ansi(&std::fs::read_to_string(path)?));
    }
    if let Some(pane) = source_pane {
        let herdr = Herdr::from_env();
        let unwrapped = herdr.pane_read(pane, cli.lines)?;
        let wrapped_ansi = herdr.pane_read_ansi(pane, cli.lines).unwrap_or_default();
        if wrapped_ansi.trim().is_empty() {
            return Ok(Buffer::from_text(&unwrapped));
        }
        return Ok(Buffer::from_reads(&unwrapped, &wrapped_ansi));
    }
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return Ok(Buffer::from_ansi(&io::read_to_string(stdin)?));
    }
    Err(io::Error::other(
        "no input: pass --input FILE, run inside herdr, or pipe text on stdin",
    ))
}

/// Run the interactive loop; returns whether anything was copied so the
/// caller can hold the pane open for the OSC 52 forwarding grace.
fn run(app: &mut App) -> io::Result<bool> {
    let _guard = ui::TerminalGuard::enter()?;
    let mut out = io::BufWriter::new(io::stdout());
    let (w, h) = crossterm::terminal::size()?;
    app.set_size(w as usize, h as usize);
    let mut copied = false;
    ui::draw(&mut out, app)?;
    while !app.quit {
        match event::read()? {
            Event::Key(k) if k.kind != KeyEventKind::Release => {
                if let Some(Effect::Copy(text)) = app.handle_key(k) {
                    match osc52::emit(&text) {
                        Ok(false) => copied = true,
                        Ok(true) => {
                            copied = true;
                            app.message = Some(format!(
                                "copied {} chars (large; terminal may truncate)",
                                text.chars().count()
                            ));
                        }
                        Err(e) => app.message = Some(format!("copy failed: {e}")),
                    }
                }
            }
            Event::Mouse(m) => app.handle_mouse(m),
            Event::Resize(w, h) => app.set_size(w as usize, h as usize),
            _ => {}
        }
        if !app.quit {
            ui::draw(&mut out, app)?;
        }
    }
    Ok(copied)
}

/// Extract loop; the resulting effect runs after the terminal is
/// restored so errors stay visible and send-text hits a live pane.
fn run_extract(app: &mut ExtractApp) -> io::Result<Option<ExtractEffect>> {
    let _guard = ui::TerminalGuard::enter()?;
    let mut out = io::BufWriter::new(io::stdout());
    let (w, h) = crossterm::terminal::size()?;
    app.set_size(w as usize, h as usize);
    ui::draw_extract(&mut out, app)?;
    while !app.quit {
        match event::read()? {
            Event::Key(k) if k.kind != KeyEventKind::Release => {
                if let Some(effect) = app.handle_key(k) {
                    return Ok(Some(effect));
                }
            }
            Event::Resize(w, h) => app.set_size(w as usize, h as usize),
            _ => {}
        }
        if !app.quit {
            ui::draw_extract(&mut out, app)?;
        }
    }
    Ok(None)
}

fn extract_mode(buf: &Buffer, source_pane: Option<&str>) -> io::Result<()> {
    let mut app = ExtractApp::new(&buf.text());
    match run_extract(&mut app)? {
        Some(ExtractEffect::Copy(text)) => {
            if osc52::emit(&text)? {
                eprintln!("copied (large; terminal may truncate)");
            }
            std::thread::sleep(osc52::FORWARD_GRACE);
        }
        Some(ExtractEffect::Insert(text)) => {
            let pane =
                source_pane.ok_or_else(|| io::Error::other("no source pane to insert into"))?;
            Herdr::from_env().send_text(pane, &text)?;
        }
        None => {}
    }
    Ok(())
}

fn copy_mode(buf: Buffer, start_search: bool) -> io::Result<()> {
    let mut app = App::new(buf, start_search);
    if run(&mut app)? {
        std::thread::sleep(osc52::FORWARD_GRACE);
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = match parse_args(std::env::args().skip(1)) {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("herdr-copy-search: {e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let source_pane = herdr::resolve_source_pane(cli.pane.as_deref(), |k| {
        std::env::var(k).ok().filter(|v| !v.is_empty())
    });
    let buf = match acquire_buffer(&cli, source_pane.as_deref()) {
        Ok(buf) => buf,
        Err(e) => {
            eprintln!("herdr-copy-search: {e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match cli.mode {
        RunMode::Extract => extract_mode(&buf, source_pane.as_deref()),
        RunMode::Copy => copy_mode(buf, false),
        RunMode::Search => copy_mode(buf, true),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("herdr-copy-search: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn defaults_to_copy_mode() {
        let cli = parse(&[]).unwrap();
        assert_eq!(cli.mode, RunMode::Copy);
        assert_eq!(cli.input, None);
        assert_eq!(cli.pane, None);
        assert_eq!(cli.lines, DEFAULT_LINES);
    }

    #[test]
    fn parses_mode_and_input() {
        let cli = parse(&["--mode", "search", "--input", "/tmp/x"]).unwrap();
        assert_eq!(cli.mode, RunMode::Search);
        assert_eq!(cli.input, Some(PathBuf::from("/tmp/x")));
    }

    #[test]
    fn parses_pane_and_lines() {
        let cli = parse(&["--pane", "w8:p1", "--lines", "500"]).unwrap();
        assert_eq!(cli.pane.as_deref(), Some("w8:p1"));
        assert_eq!(cli.lines, 500);
    }

    #[test]
    fn rejects_unknown_arguments_and_modes() {
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--mode", "bogus"]).is_err());
        assert!(parse(&["--mode"]).is_err());
        assert!(parse(&["--lines", "abc"]).is_err());
    }
}
