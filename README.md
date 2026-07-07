# herdr-copy-search

tmux-style copy mode with incremental search for [herdr](https://herdr.dev)
pane scrollback, plus tmux-copycat-style predefined pattern search and
extrakto-style token extraction. All copying goes through OSC 52, so it
works over SSH without any clipboard helper on the remote host.

herdr has a native copy mode but no scrollback search yet (see
upstream [discussion #563](https://github.com/ogulcancelik/herdr/discussions/563));
this plugin fills that gap until it lands natively - the planned exit
is written down in [docs/ROADMAP.md](docs/ROADMAP.md).

## Status

Personal tool, built with heavy AI assistance (Claude Code). I review
what ships and use it daily over SSH, but it comes with no warranty
and no support commitment: issues and PRs are welcome and may still
go unanswered. Expect tmux-like behavior, not tmux-grade maturity.

## Install

```sh
herdr plugin install qq88976321/herdr-copy-search   # from a git host
# or for local development:
herdr plugin link /path/to/herdr-copy-search
cargo build --release --locked                   # link does not run build steps
```

Bind keys in `~/.config/herdr/config.toml` (reload with
`herdr server reload-config`):

```toml
[[keys.command]]
key = "prefix+["
type = "shell"
command = "herdr plugin pane open --plugin copy-search --entrypoint copy"

[[keys.command]]
key = "prefix+/"
type = "shell"
command = "herdr plugin pane open --plugin copy-search --entrypoint search"

[[keys.command]]
key = "prefix+tab"
type = "shell"
command = "herdr plugin pane open --plugin copy-search --entrypoint extract"
```

If your herdr build supports `type = "plugin_action"` bindings, the
plugin also exposes `copy-search.copy`, `copy-search.search`, and
`copy-search.extract` actions that open the same panes.

## Copy / search mode

Opens an overlay over the focused pane showing its recent scrollback
(the herdr server caps pane reads at about 1000 lines). The `search`
entrypoint is the same view opened with the search prompt active.

| Keys | Action |
| --- | --- |
| `h j k l`, arrows | move (a column count is kept across short lines) |
| `w b e`, `{ }` | word and paragraph motions |
| `0 ^ $`, `g G` | line start/end, buffer top/bottom |
| `ctrl+f/b/d/u`, PgUp/PgDn | page and half-page |
| `/` `?` | incremental regex search down / up (invalid regex falls back to literal) |
| in prompt: `Left/Right`, `Home/End`, `Delete` | move the cursor and edit the query (`ctrl+a/e` jump, `ctrl+u` clears) |
| `n` `N` | next / previous match in search direction |
| `S` | toggle smartcase (default on: an all-lowercase query ignores case, any uppercase is case-sensitive; off is always case-sensitive) |
| `v` `V` | charwise / linewise selection |
| `iw aw iW aW` | select the word / WORD text object under the cursor (works after `v` or on its own) |
| `y` | copy selection, or the match under the cursor |
| `Enter` | copy and exit |
| `p` | pattern menu: `[u]rl [f]ile [g]it-sha [i]p [d]igits [q]uoted` |
| `alt+u/f/g/i/d/q` | activate a pattern directly |
| mouse | wheel scrolls, click moves, drag selects |
| `q`, `Esc` | exit (Esc clears the selection first) |

Pattern searches run backward from the cursor like tmux-copycat: the
nearest match above is selected and `n` walks up through older output.

Add your own patterns, or disable defaults, in the plugin config file
(path from `herdr plugin config-dir copy-search`). A `pattern_<key>`
entry appears in the menu and on `alt+<key>`; an empty value removes a
default; reusing a default's key replaces its regex:

```toml
# $HERDR_PLUGIN_CONFIG_DIR/config.toml
pattern_t = "\\bTODO\\b"   # p then t, or alt+t
pattern_j = "\\{[^}]*\\}"  # a JSON-ish object
pattern_u = ""             # disable the built-in url pattern
```

Values are taken verbatim between the quotes (no TOML escape
processing). An invalid regex falls back to a literal search, matching
the incremental-search behavior.

## Extract mode

extrakto-style picker over the same scrollback: tokens (or whole lines)
are listed newest first and filtered with a built-in fuzzy matcher.
Like `fzf --height`, the picker occupies only the bottom of the overlay
(default 40%); the pane content stays visible above it in its original
colors.

Popup height: pass `--height PCT` in the pane command, or set it in
the plugin config file (path from `herdr plugin config-dir copy-search`):

```toml
# $HERDR_PLUGIN_CONFIG_DIR/config.toml
extract_height = 40
```

| Keys | Action |
| --- | --- |
| type / Backspace | edit the fuzzy query |
| `Left/Right`, `Home/End`, `Delete` | move the cursor within the query (`ctrl+a/e` jump) |
| `Up`/`Down`, `ctrl+p/n` | move the selection |
| `PgUp/PgDn`, wheel over the backdrop | scroll the pane output above the popup (wheel over the list moves the selection) |
| `Enter` | insert the pick into the source pane (`herdr pane send-text`) |
| `Tab` | copy the pick via OSC 52 |
| `ctrl+t` | toggle word / line granularity |
| `ctrl+u` | clear the query |
| `Esc` | clear the query, then exit |

## Development

```sh
just gate                                # fmt + clippy + test + release build
just run copy                            # fixture run outside herdr
herdr plugin pane open --plugin copy-search --entrypoint copy   # inside herdr
just release patch                       # bump + sync manifests + local tag
```

`--pane ID` overrides the source pane, `--lines N` the read size.
Outside herdr you can also pipe text on stdin; key events then come
from `/dev/tty`. Releases are local-only (`cargo-release` with
`publish = false`, `push = false`); `herdr-plugin.toml` is version-
synced automatically.

Where things are headed and how work gets done here:
[docs/ROADMAP.md](docs/ROADMAP.md) (evidence-based roadmap, archive
conditions) and [docs/checklists/](docs/checklists/) (step-by-step
procedures: publishing, CI, releases, demo recording, feature loop,
upstream watch). README demo GIFs are scripted in [demo/](demo/) as
vhs tapes so they can be regenerated after any UI change.

## Design notes

- Copying emits OSC 52 to `/dev/tty` and the process sleeps 200 ms
  before exiting so herdr can forward the write to the outer terminal.
  Payloads beyond ~100 KB of base64 may be truncated by the terminal;
  the status line warns when that can happen.
- The source pane is resolved from `--pane`, then
  `HERDR_ACTIVE_PANE_ID`, then the `focused_pane_id` field of
  `HERDR_PLUGIN_CONTEXT_JSON`. `HERDR_PANE_ID` is the overlay pane
  itself and is deliberately ignored.
- Colors: buffer rows come from `pane read --source recent --format
  ansi` (styled, wrapped at the pane width) so the overlay looks like
  the frozen pane; a second `recent-unwrapped --format text` read
  aligns them into soft-wrap flags so yanks rejoin wrapped long lines.
  (`recent-unwrapped --format ansi` returns empty on herdr 0.7.1 once
  scrollback grows, hence the two-read composition.) Known limits:
  searches do not match across a wrap boundary, and if the pane
  changes between the two reads the wrap joining degrades to hard
  breaks for that snapshot.
- The bottom row is a persistent mode row (`[copy]` / `[search]`)
  holding the prompt, pattern menu, and messages, like herdr's native
  copy mode; match counts and the cursor position stay in the
  tmux-style top-right indicator.
- Opening the overlay keeps the source pane's screen in place: a third
  `visible` read anchors the bottom-most view to the rows the pane was
  showing, so a partly filled screen stays at the top with blank rows
  below and scrolling back down restores the original screen. When the
  pane changes between reads, the anchor degrades to tail alignment.
- No serde or toml crates: herdr's context JSON and the plugin config
  are flat, so small string-field extractors are enough.
- Extract mode follows extrakto's convention (Enter inserts, Tab
  copies); copy mode follows tmux (Enter copies and exits).
