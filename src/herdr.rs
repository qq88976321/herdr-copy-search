use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Thin wrapper over the herdr CLI, which is the plugin API surface.
pub struct Herdr {
    bin: PathBuf,
}

/// The three snapshots of a source pane that a session starts from.
pub struct PaneReads {
    /// Logical lines, SGR-styled (`recent-unwrapped --format ansi`).
    pub unwrapped_ansi: String,
    /// Rows wrapped at the pane width, SGR-styled (`recent --format
    /// ansi`); empty when that read failed.
    pub wrapped_ansi: String,
    /// The current screen as plain rows (`visible --format text`, the
    /// server trims trailing blank rows); empty when that read failed.
    pub visible: String,
}

impl Herdr {
    pub fn from_env() -> Self {
        let bin = std::env::var_os("HERDR_BIN_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("herdr"));
        Herdr { bin }
    }

    fn spawn_read(
        &self,
        pane: &str,
        lines: usize,
        source: &str,
        format: &str,
    ) -> io::Result<Child> {
        Command::new(&self.bin)
            .args([
                "pane",
                "read",
                pane,
                "--source",
                source,
                "--lines",
                &lines.to_string(),
                "--format",
                format,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }

    fn finish_read(child: Child, pane: &str) -> io::Result<String> {
        let out = child.wait_with_output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "herdr pane read {pane} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        String::from_utf8(out.stdout).map_err(io::Error::other)
    }

    /// Issue the three reads at once and collect them together, so the
    /// startup cost is the slowest read rather than their sum. The herdr
    /// server caps each at ~1000 lines regardless of `lines`. Only the
    /// unwrapped read is required; the other two degrade to empty.
    ///
    /// The scrollback sources are read as `ansi` even where only text is
    /// needed: a `text` read of `recent`/`recent-unwrapped` on an idle
    /// TUI-agent pane (Claude Code and friends) makes herdr page through
    /// the alternate screen to collect history, which takes 0.5-1.1 s and
    /// returns nothing beyond the screen (herdr #2387, #2426). `ansi`
    /// skips that and stripping SGR locally is free.
    pub fn read_pane(&self, pane: &str, lines: usize) -> io::Result<PaneReads> {
        let unwrapped = self.spawn_read(pane, lines, "recent-unwrapped", "ansi")?;
        let wrapped = self.spawn_read(pane, lines, "recent", "ansi");
        let visible = self.spawn_read(pane, lines, "visible", "text");
        let unwrapped_ansi = Self::finish_read(unwrapped, pane)?;
        let wrapped_ansi = wrapped
            .and_then(|child| Self::finish_read(child, pane))
            .unwrap_or_default();
        let visible = visible
            .and_then(|child| Self::finish_read(child, pane))
            .unwrap_or_default();
        Ok(PaneReads {
            unwrapped_ansi,
            wrapped_ansi,
            visible,
        })
    }

    /// Type `text` into a pane, as if entered at its terminal.
    pub fn send_text(&self, pane: &str, text: &str) -> io::Result<()> {
        let out = Command::new(&self.bin)
            .args(["pane", "send-text", pane, text])
            .output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "herdr pane send-text {pane} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(())
    }
}

/// The pane whose scrollback we operate on. Spike-verified order:
/// explicit --pane, then HERDR_ACTIVE_PANE_ID (set for keys.command
/// bindings), then the context JSON's focused_pane_id (set for plugin
/// panes and actions). HERDR_PANE_ID is the plugin pane itself, never
/// the source, so it is deliberately not consulted.
pub fn resolve_source_pane(
    cli: Option<&str>,
    env: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    if let Some(pane) = cli {
        return Some(pane.to_string());
    }
    if let Some(pane) = env("HERDR_ACTIVE_PANE_ID") {
        return Some(pane);
    }
    env("HERDR_PLUGIN_CONTEXT_JSON").and_then(|json| json_str_field(&json, "focused_pane_id"))
}

/// Minimal `"key":"value"` extractor for herdr's flat context JSON.
/// Values with escaped quotes are not handled; pane and workspace ids
/// are plain tokens like "w8:p1".
pub fn json_str_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let rest = &json[json.find(&needle)? + needle.len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTX: &str = r#"{"workspace_id":"w8","tab_id":"w8:t1","focused_pane_id":"w8:p1","focused_pane_cwd":"/home/x","invocation_source":"api"}"#;

    #[test]
    fn json_str_field_extracts_flat_fields() {
        assert_eq!(
            json_str_field(CTX, "focused_pane_id").as_deref(),
            Some("w8:p1")
        );
        assert_eq!(json_str_field(CTX, "workspace_id").as_deref(), Some("w8"));
        assert_eq!(json_str_field(CTX, "missing"), None);
    }

    #[test]
    fn json_str_field_allows_spacing() {
        let json = r#"{ "focused_pane_id" : "w1:p2" }"#;
        assert_eq!(
            json_str_field(json, "focused_pane_id").as_deref(),
            Some("w1:p2")
        );
    }

    #[test]
    fn cli_pane_wins_over_env() {
        let env = |_: &str| Some("env-pane".to_string());
        assert_eq!(
            resolve_source_pane(Some("cli-pane"), env).as_deref(),
            Some("cli-pane")
        );
    }

    #[test]
    fn active_pane_id_wins_over_context_json() {
        let env = |k: &str| match k {
            "HERDR_ACTIVE_PANE_ID" => Some("w8:pA".to_string()),
            "HERDR_PLUGIN_CONTEXT_JSON" => Some(CTX.to_string()),
            _ => None,
        };
        assert_eq!(resolve_source_pane(None, env).as_deref(), Some("w8:pA"));
    }

    #[test]
    fn context_json_is_the_fallback() {
        let env = |k: &str| match k {
            "HERDR_PLUGIN_CONTEXT_JSON" => Some(CTX.to_string()),
            _ => None,
        };
        assert_eq!(resolve_source_pane(None, env).as_deref(), Some("w8:p1"));
    }

    #[test]
    fn no_sources_resolves_to_none() {
        assert_eq!(resolve_source_pane(None, |_| None), None);
    }
}
