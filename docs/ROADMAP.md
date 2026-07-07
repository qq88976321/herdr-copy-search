# Roadmap

Goal: the best copy / search / extract experience on herdr until herdr
ships native scrollback search - then exit gracefully (see "Archive
conditions" at the bottom).

Provenance: researched 2026-07-07 (herdr v0.7.1) from upstream
trackers, tmux/zellij/wezterm/kitty ecosystems, and community threads;
every claim cites its source. Re-verify time-sensitive facts with
docs/checklists/CL-07-upstream-watch.md before acting on them.

How to use this file: pick the highest unfinished item whose
dependencies are done, follow its checklist (docs/checklists/), meet
the done criteria, run `just gate`, commit, mark the item done here.
Feature work follows docs/checklists/CL-06-feature-development.md.

Status legend: [ ] todo, [x] done, [~] partial/in progress,
[!] blocked upstream.

## Positioning (why this plugin should exist at all)

- herdr has a native copy mode (`prefix+[`, upstream issue #231) but
  NO scrollback search: no `/` `?` in copy mode and no search
  subcommand. Users ask for it in unanswered Discussion #563
  ("Add / and ? search in copy mode").
- Among 111 repos tagged `herdr-plugin` (2026-07), no other plugin
  does scrollback regex search with in-place highlighting. Closest
  neighbor is rmarganti/herdr-pluck: thumbs-style hints over VISIBLE
  content only.
- The core design (keyboard-driven selection over scrollback) matches
  the single most-demanded capability in neighboring ecosystems:
  zellij "keyboard select and copy" issue #947 (88 reactions) and
  word-boundary selection #1258 (147 reactions), wezterm copy-mode
  motions #4471 (21 reactions).

So: unique niche, real demand, and an upstream that may eventually
absorb it. Optimize for usefulness now, low maintenance cost, and a
clean exit.

## Phase 0 - publish and trust

| ID | Item | Effort | Checklist |
|----|------|--------|-----------|
| P0.1 | [ ] Publish to GitHub | S | CL-01 |
| P0.2 | [x] AI-assistance disclosure in README | S | - |
| P0.3 | [ ] Submit to awesome-herdr | S | CL-01 |

- P0.1: public repo, topics (`herdr-plugin`, `herdr`, `copy-mode`,
  `tmux`, `rust`, `terminal`), description, verify
  `herdr plugin install <owner>/herdr-copy-search` works end to end.
  Done when: install-from-GitHub works on a clean machine/config.
- P0.2: soft disclosure, per 2025-2026 community norms research:
  backlash targets unreviewed, un-owned code, not AI assistance
  itself (see RedMonk "AI slopageddon" 2026-02; Ghostty is itself
  heavily AI-assisted with a strict disclosure policy). The respected
  pattern (Simon Willison) is: state AI assistance + human review +
  daily personal use + support expectations. Wording lives in
  README "Status" section. Avoid the phrase "vibe coded".
- P0.3: PR to yigitkonur/awesome-herdr after P0.1. Done when listed.
- License note: herdr is dual AGPL-3.0/commercial; this plugin is a
  separate MIT work that drives the `herdr` CLI as a subprocess and
  links nothing from it, so MIT remains correct.

## Phase 1 - engineering foundation

| ID | Item | Effort | Depends | Checklist |
|----|------|--------|---------|-----------|
| P1.1 | [~] CI pipeline (authored, UNVERIFIED) | S | P0.1 | CL-02 |
| P1.2 | [~] Release artifacts workflow (authored, UNVERIFIED) | S | P1.1 | CL-03 |
| P1.3 | [~] Lints + MSRV (lints added; MSRV unverified) | S | - | CL-05 |
| P1.4 | [~] Reproducible README demos (tapes authored, UNVERIFIED) | M | - | CL-04 |
| P1.5 | [x] Dev process documented | S | - | CL-06 |

- P1.1: .github/workflows/ci.yml mirrors `just gate` (fmt, clippy
  -D warnings, test+build on ubuntu+macos, --locked) plus cargo-audit.
  Chosen stack per 2026 idioms: actions/checkout@v4,
  dtolnay/rust-toolchain, Swatinem/rust-cache@v2,
  taiki-e/install-action. Deliberately NOT cargo-dist (its steward
  axo wound down; treat as risky). Done when: all jobs green on
  GitHub and the UNVERIFIED header comment is removed.
- P1.2: .github/workflows/release.yml attaches linux/macos binaries
  to the GitHub Release on a v* tag via
  taiki-e/upload-rust-binary-action. Note: plugin installs build from
  source (herdr-plugin.toml [[build]]), so release binaries are a
  convenience, not the install path. Versioning stays local
  (cargo-release; `just release`, USER-ONLY per CLAUDE.md).
- P1.3: Cargo.toml [lints.rust]/[lints.clippy] tables (RFC 3389
  style) with a conservative set that passes the gate today;
  rust-version = "1.74" is an UNVERIFIED floor (the [lints] table
  itself needs cargo >= 1.74) - CL-05 verifies with cargo-msrv and
  enables the msrv CI job. Formatter: rustfmt defaults, no
  rustfmt.toml (bat/fd/zoxide convention). Track stable in CI, no
  rust-toolchain.toml pin.
- P1.4: vhs tapes (demo/*.tape) drive the binary against
  fixtures/sample.txt in a plain terminal - keystrokes-as-code, so an
  AI agent can regenerate GIFs after any UI change (this is how
  ratatui maintains its example GIFs). Done when: GIFs generated,
  embedded in README, and regeneration procedure verified once.
- P1.5: CL-06 encodes the repo dev loop (TDD, gate, manual TTY test,
  demo regeneration, conventional commit).

## Phase 2 - features, ranked by evidence

Evidence = reaction-weighted demand across tmux/copycat/extrakto/
thumbs/fingers/zellij/wezterm/kitty trackers (details in Evidence
appendix). Follow CL-06 for every item. Ordering within the phase is
a recommendation; re-rank freely on personal need - the only real
user is the author.

| ID | Item | Effort | Evidence rank |
|----|------|--------|---------------|
| F1 | [~] User-defined patterns via plugin config, TTY check pending | M | #2 |
| F2 | [ ] Extract: jump-to-match (land in copy mode at the token) | S/M | #11 |
| F3 | [ ] Open actions: file:line in $EDITOR, URL in browser | M/L | #4 |
| F4 | [ ] Search across soft-wrap boundaries | M | #10 |
| F5 | [~] Word text objects (viw/vaw/viW/vaW), TTY check pending | S | #1 gap |
| F6 | [ ] Extract: multi-select (Tab-mark several tokens) | S/M | #13 |
| F7 | [!] Copy last command output / prompt jump (OSC 133) | L | #5 |
| F8 | [~] Smartcase toggle in search (`S`), TTY check pending | S | weak |
| R1 | [ ] Audit ~16 unwrap/expect/panic sites in src | S | robustness |

- F1: every mature tool converged on user regexes in config (extrakto
  redirects all pattern requests to its config; wezterm
  quick_select_patterns; kitty hints custom regex; thumbs #44;
  fingers #19). Design constraint: config.rs is a flat-TOML parser
  (no arrays, stops at first [table]); use flat keys, e.g.
  `pattern_j = "regex"` adds pattern with menu key `j`, and
  `pattern_u = ""` disables a default. Done when: custom pattern
  usable from menu and alt+key, defaults overridable, README table
  updated, unit tests for parsing and menu.
- F2: from the extract list, a key (e.g. ctrl+j) jumps into copy mode
  with the cursor on that token's position instead of inserting or
  copying - "navigate to the found string, in context" (extrakto
  #130), "jump into the selected position instead of just yanking"
  (thumbs #146). Both modes already share the buffer, which makes
  this cheap here and impossible for most competitors. Done when:
  round trip extract -> copy mode -> refine selection -> yank works
  and is in the README key table.
- F3: the killer action in kitty hints is "open file at line in
  $EDITOR"; tmux-open and extrakto #15/#20 show the same demand.
  Design questions to settle first (see CL-06 brainstorm step): the
  overlay cannot exec $EDITOR itself for the source pane - likely
  `herdr pane send-text` of an editor command, or a config-defined
  opener command; also check herdr link handlers as the URL path.
  Done when: pattern/extract pick can open in editor/browser with a
  config-defined command, documented caveats.
- F4: known limit documented in README design notes (searches do not
  match across a wrap boundary); real demand: tmux #966, copycat
  #59/#94, zellij #2853. The two-read composition already aligns
  soft-wrap flags for yanks; extend matching to logical
  (wrap-joined) lines and map hits back to display rows. Done when:
  a URL split across two display rows is findable and yankable as
  one match; unit tests cover the row mapping.
- F5: wezterm #4471 (21 reactions) asks for exactly this in a copy
  mode. We have w/b/e motions already; add viw/vaw/viW/vaW. Done
  when: text objects work in charwise selection + tests.
- F6: thumbs multi-select is praised; fingers #169. Tab marks
  multiple tokens in extract, Enter/copy acts on all (joined by
  newline or space; decide in brainstorm). Done when: multi-pick
  insert and copy work + README updated.
- F7: BLOCKED-ish: needs prompt marks. Investigate first (CL-07 item)
  whether `herdr pane read --source detection` or any OSC 133
  exposure exists; tmux/zellij/kitty all grew this class of feature
  (tmux #3102, zellij #899, kitty show_last_command_output). If herdr
  exposes nothing, leave blocked; do NOT try to parse prompts
  heuristically.
- F8: weak evidence (no high-reaction ask found); do only if it
  personally itches. Default should be smartcase if built.
- R1: audit unwrap/expect/panic in non-test code paths; each site
  either becomes an error path or gets a comment stating why it
  cannot fail. TUI must never panic mid-alt-screen (leaves the
  terminal broken).

## Phase 3 - distribution and polish

| ID | Item | Effort | Depends |
|----|------|--------|---------|
| P3.1 | [ ] README media embedded (GIFs from P1.4) | S | P1.4 |
| P3.2 | [ ] Install path verified from GitHub + version badge | S | P0.1, P1.1 |
| P3.3 | [ ] Optional: announce (herdr Discussions, r/commandline) | S | P3.1 |

P3.3 is optional and only after P0-P2 core items feel solid; an
announcement invites users, users invite support expectations - the
disclosure section sets those expectations first.

## In-place search (no separate pane/layout) - status: BLOCKED UPSTREAM

The wish: search like native copy mode, no overlay pane. As of herdr
v0.7.1 the plugin docs state "No API exists for extending or
overriding copy mode in v1", and plugin surfaces are panes
(placement: overlay/split/tab/zoomed). Overlay is already the least
disruptive: it covers only the focused pane and restores focus and
zoom on close; this plugin additionally anchors the opening view to
the pane's visible screen so it LOOKS in-place.

Unblock triggers (checked by CL-07):
- native `/` `?` search ships in herdr copy mode -> see Archive
  conditions instead
- a copy-mode extension/override API ships -> port this plugin's
  search into it (major version)
- floating/popup plugin surfaces ship (upstream #785, #812) -> minor
  UX win for extract mode

## Continuous - upstream watch

Run docs/checklists/CL-07-upstream-watch.md quarterly, or before
starting any Phase 2 item, or when herdr updates beyond a patch
release. Log findings in docs/upstream-log.md. Empirical facts that
can silently change: the ~1000-line pane read cap (undocumented
upstream), OSC 52 forwarding timing (the 200 ms exit grace), overlay
placement behavior, plugin manifest schema.

## Archive conditions

This repo exists to fill an upstream gap. Archive it without regret
when the gap closes. Decision table - evaluate during CL-07 runs:

| # | Condition | How to check | Action |
|---|-----------|--------------|--------|
| A1 | herdr STABLE release ships `/` `?` search in copy mode with match highlighting | herdr CHANGELOG + Discussion #563 | Try native for 2 weeks. If it covers daily use: archive. If patterns/extract still add value: strip to the surviving modes, note in README |
| A2 | Copy-mode extension API ships | plugin docs sentence "No API exists for extending or overriding copy mode" disappears | Either port (new major) or archive in favor of a native-integrated successor |
| A3 | Another maintained plugin reaches scrollback-search parity | awesome-herdr + `herdr-plugin` topic search | Archive, link to it from README |
| A4 | herdr breaks the plugin contract (pane read semantics, overlay placement) and fixing costs more than the plugin is worth | CI red + CL-07 | Pin last working herdr version in README, mark maintenance-only; archive if unused |
| A5 | Author has not used it for 6 months | be honest | Archive |

Archive procedure: set repo archived on GitHub, add a final README
banner naming the reason and the replacement, tag a final release.

## Evidence appendix

herdr upstream (verified 2026-07-07, herdr v0.7.1):
- repo: https://github.com/ogulcancelik/herdr (13.2k stars, created
  2026-03, stable releases every 1-2 weeks + near-daily previews)
- native copy mode: issue #231 (shipped); no native search: no `/`
  `?` in config docs key tables, no search subcommand in CLI ref
- search demand: https://github.com/ogulcancelik/herdr/discussions/563
  (open, unanswered, no maintainer commitment, no upstream ROADMAP)
- no copy-mode plugin API: https://herdr.dev/docs/plugins/ ("No API
  exists for extending or overriding copy mode in v1")
- adjacent upstream issues: #680 copy-mode follows live output, #970
  vim word boundaries, #1000 CJK glyph stops, #785/#812 floating
  surfaces, #893 plugin registry clobber, #978 --lines 0 empty
- OSC 52: forwarded from pane children (#30); herdr itself prefers
  native clipboard tools then OSC 52 (#333)
- competitor: https://github.com/rmarganti/herdr-pluck (visible-only
  hints); list: https://github.com/yigitkonur/awesome-herdr
- HN launch thread: https://news.ycombinator.com/item?id=48714802

Cross-ecosystem demand (reaction counts as of 2026-07-07):
- keyboard selection: zellij #1258 (147), #947 (88), #2989; tmux #140
  (28); wezterm #4471 (21)
- custom patterns: extrakto #95 (closed-not-planned -> config);
  thumbs #44; fingers #19, #117; wezterm quick_select_patterns docs;
  kitty hints docs
- open/file:line actions: extrakto #15, #20, #130; zellij #2527 (16);
  wezterm #5534 (9); tmux-plugins/tmux-open; kitty hints docs
- OSC 133 / last-command-output: tmux #3064, #3102, #5237; zellij
  #899 (17), #4414; kitty show_last_command_output
- OSC 52 pain: zellij #3013 (28), #2647 (31); tmux wiki Clipboard
- navigate-with-highlights gap: tmux #972 (open); copycat #69, #127
- incremental search: copycat #70 (10); tmux #895 (14)
- perf on large scrollback: thumbs #88; copycat #129 (13), #123
- wrapped-line matching: tmux #966; copycat #59, #94; zellij #2853 (13)
- jump-to-match: thumbs #146; extrakto #130; roosta/tmux-fuzzback
- multi-select: fingers #169; thumbs README
- insert/copy/paste flow: extrakto #2; wezterm quickselect docs
  (lowercase copies, uppercase copies+pastes)
- weak evidence (deprioritized): case toggle, rectangular selection

AI disclosure norms:
- policy catalog: https://github.com/melissawm/open-source-ai-contribution-policies
- curl policy: https://github.com/curl/curl/blob/master/docs/CONTRIBUTE.md
- the respected disclosure pattern: https://simonwillison.net/2026/May/6/vibe-coding-and-agentic-engineering/
- backlash analysis: https://redmonk.com/kholterhoff/2026/02/03/ai-slopageddon-and-the-oss-maintainers/

Tooling choices:
- vhs: https://github.com/charmbracelet/vhs (+ vhs-action; ratatui
  regenerates its example GIFs this way)
- CI idioms: https://rustprojectprimer.com/ci/github.html
- release binaries: https://github.com/taiki-e/upload-rust-binary-action
- cargo-dist avoided: renamed to dist 2024-10, steward axo wound down
  (axo.dev parked for sale 2026)
