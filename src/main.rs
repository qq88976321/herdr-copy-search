use crossterm::event::{self, Event, KeyEventKind};
use herdr_copy_search::app::{App, Effect};
use herdr_copy_search::buffer::Buffer;
use herdr_copy_search::herdr::{self, Herdr};
use herdr_copy_search::{osc52, ui};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str =
    "usage: herdr-copy-search [--mode copy|search] [--pane ID] [--lines N] [--input FILE]";

/// The herdr server caps pane reads at ~1000 lines (spike-verified),
/// so asking for more only future-proofs against a raised cap.
const DEFAULT_LINES: usize = 1000;

#[derive(Debug, PartialEq, Eq)]
enum RunMode {
    Copy,
    Search,
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
fn acquire_input(cli: &Cli, source_pane: Option<&str>) -> io::Result<String> {
    if let Some(path) = &cli.input {
        return std::fs::read_to_string(path);
    }
    if let Some(pane) = source_pane {
        return Herdr::from_env().pane_read(pane, cli.lines);
    }
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return io::read_to_string(stdin);
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
    let text = match acquire_input(&cli, source_pane.as_deref()) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("herdr-copy-search: {e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let mut app = App::new(
        Buffer::from_text(&text),
        matches!(cli.mode, RunMode::Search),
    );
    match run(&mut app) {
        Ok(copied) => {
            if copied {
                std::thread::sleep(osc52::FORWARD_GRACE);
            }
            ExitCode::SUCCESS
        }
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
