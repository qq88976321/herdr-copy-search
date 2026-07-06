# herdr-copy-search

tmux-style copy mode with incremental search for [herdr](https://herdr.dev)
pane scrollback, plus tmux-copycat-style predefined pattern search and
extrakto-style token extraction. All copying goes through OSC 52, so it
works over SSH without any clipboard helper on the remote host.

## Install

```sh
herdr plugin install <owner>/herdr-copy-search   # from a git host
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
| `n` `N` | next / previous match in search direction |
| `v` `V` | charwise / linewise selection |
| `y` | copy selection, or the match under the cursor |
| `Enter` | copy and exit |
| `p` | pattern menu: `[u]rl [f]ile [g]it-sha [i]p [d]igits [q]uoted` |
| `alt+u/f/g/i/d/q` | activate a pattern directly |
| mouse | wheel scrolls, click moves, drag selects |
| `q`, `Esc` | exit (Esc clears the selection first) |

Pattern searches run backward from the cursor like tmux-copycat: the
nearest match above is selected and `n` walks up through older output.

## Extract mode

extrakto-style picker over the same scrollback: tokens (or whole lines)
are listed newest first and filtered with a built-in fuzzy matcher.

| Keys | Action |
| --- | --- |
| type / Backspace | edit the fuzzy query |
| `Up`/`Down`, `ctrl+p/n` | move the selection |
| `Enter` | insert the pick into the source pane (`herdr pane send-text`) |
| `Tab` | copy the pick via OSC 52 |
| `ctrl+t` | toggle word / line granularity |
| `ctrl+u` | clear the query |
| `Esc` | clear the query, then exit |

## Development

```sh
cargo test
cargo run -- --input fixtures/sample.txt --mode copy     # outside herdr
herdr plugin pane open --plugin copy-search --entrypoint copy   # inside herdr
```

`--pane ID` overrides the source pane, `--lines N` the read size.
Outside herdr you can also pipe text on stdin; key events then come
from `/dev/tty`.

## Design notes

- Copying emits OSC 52 to `/dev/tty` and the process sleeps 200 ms
  before exiting so herdr can forward the write to the outer terminal.
  Payloads beyond ~100 KB of base64 may be truncated by the terminal;
  the status line warns when that can happen.
- The source pane is resolved from `--pane`, then
  `HERDR_ACTIVE_PANE_ID`, then the `focused_pane_id` field of
  `HERDR_PLUGIN_CONTEXT_JSON`. `HERDR_PANE_ID` is the overlay pane
  itself and is deliberately ignored.
- No serde: herdr's context JSON is flat, so a small string-field
  extractor is enough.
- Extract mode follows extrakto's convention (Enter inserts, Tab
  copies); copy mode follows tmux (Enter copies and exits).
