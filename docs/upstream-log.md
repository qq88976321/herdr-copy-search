# Upstream watch log

Append-only log of CL-07 runs (newest first). Entry template:

    ## YYYY-MM-DD (herdr vX.Y.Z installed, vX.Y.Z latest)
    - changelog: <relevant items or "nothing relevant">
    - discussion #563: <open/answered/closed>
    - copy-mode API sentence: <present/absent>
    - competitors: <changes or "none">
    - empirical contract: <re-verified/skipped (no version change)>
    - archive table: <no trigger / trigger Ax -> action taken>

## 2026-07-10 (herdr v0.7.3 installed, v0.7.3 latest; full CL-07 run)

Full CL-07 run during a roadmap re-analysis (three-way research pass:
repo inventory, tmux-ecosystem sweep, herdr upstream refresh).

- changelog: MAJOR - native copy-mode search is MERGED TO MASTER but
  UNRELEASED (issue #1230, opened 2026-07-09, closed completed
  2026-07-10; latest tag is still v0.7.3). Unreleased CHANGELOG:
  literal smart-case search with `/` and `?`, repeat with `n`/`N`,
  match highlighting, tmux-style cross-line w/b/e word motions.
  Native search is literal-only: no regex, no patterns, no extract,
  no jump/open/paste actions. #1263 (w/b/e should match tmux) closed
  the same day as part of the same work. Also noted from v0.7.2
  (previously unlogged, plugin-relevant): pane scroll metrics +
  `pane.scroll_changed` subscriptions, `herdr terminal session
  observe`/`control` (live ANSI streams), `session.snapshot`,
  `herdr api schema`. Since 0.7.0: `[[link_handlers]]` route
  ctrl-clicked URLs to a plugin action (HERDR_PLUGIN_CLICKED_URL),
  and docs state HERDR_PLUGIN_CONTEXT_JSON carries selected text and
  clicked URL (NOT yet verified empirically; roadmap F15 depends).
- discussion #563: CLOSED - folded by the maintainer into issue
  #1230. The demand thread is resolved by the native feature.
- copy-mode API sentence: ABSENT - but via a docs reorganization,
  NOT an extension API shipping. The plugins doc no longer mentions
  copy mode at all; the nearest statement is now "Runtime action
  registration and native non-terminal plugin UI are not part of
  plugin v1". The old A2 sentinel is dead; CL-07 step 4 rewritten
  this run with a new sentinel. No copy-mode extension API exists.
- competitors: two NEW visible-only tools appeared (updated
  2026-07-09): hotchpotch/herdr-tiny-fingers (tmux-fingers-style
  hints) and iurysza/termscope (open visible files/links). Both are
  visible-screen only. rmarganti/herdr-pluck unchanged (7 stars,
  listed in awesome-herdr). `herdr-plugin` topic now 136 repos (was
  111). The scrollback search + patterns + extract combo still has
  no competitor. herdr-copy-search itself is still NOT listed in
  awesome-herdr (P0.3 open).
- floating surfaces: #785/#812 are closed not_planned; the live
  successor thread is #1125 "Temporary floating popup panes for
  plugins" (open, 9 reactions) - now the watch target for the
  in-place item. Also new: #1174 (plugin installs across sessions,
  3 reactions; sibling of #893).
- OSC 133 / prompt marks: still nothing upstream (shell integration
  covers cwd reporting only); F7 stays [!] blocked.
- empirical contract: skipped (no version change since the 0.7.3
  re-check on 2026-07-08).
- archive table: NO TRIGGER YET, but A1 is now ARMED: #1230 is
  merged but A1 requires a STABLE release. When the next stable tag
  ships with it, start the two-week native trial per A1. A2 not
  triggered (docs reorg, not an API). A3 not triggered (competitors
  are visible-only).
- decision (user, 2026-07-10): differentiation posture - invest only
  in what native search will not do (regex search, patterns,
  extract, jump/open/paste actions, copy ergonomics); FREEZE
  copy-mode motion-parity work (f/F/t/T, counts, marks, `*`/`#`,
  block selection) until the A1 trial has an outcome. ROADMAP
  re-ranked accordingly (F11-F17 added).

## 2026-07-09 (repo public; CI/release verified; herdr 0.7.3)

Ad-hoc verification during a roadmap sync (not a full CL-07 run).
herdr v0.7.3 installed (0.7.1 at the baseline research).

- repo: now a PUBLIC GitHub repo (qq88976321/herdr-copy-search),
  default branch master, tags v0.1.1..v0.1.3 on origin.
- CI: `ci` workflow run on master HEAD (0ea3661) = success (GitHub API).
- release: `release` workflow on v0.1.3 = success; attached linux +
  x86_64/aarch64 macOS binaries + sha256 and git-cliff notes;
  releases/latest = v0.1.3. Historical: the v0.1.2 release run FAILED,
  but v0.1.3 is green, so the pipeline works now.
- pages: NOT enabled (has_pages=false, site 404); P3.4 deploy pending.
- empirical contract: not re-run in full; the 0.7.3 two-read trailing-
  whitespace asymmetry was recorded in CLAUDE.md on 2026-07-08. A full
  CL-07 run is still due before the next Phase-2 item.
- archive table: no trigger.

## 2026-07-07 (UX follow-up - overlay chrome and placement)

Ad-hoc verification while the author trialled the plugin in real herdr
(not a full CL-07 run). herdr v0.7.1.

- pane borders: a single global bool `[ui] pane_borders` (default true)
  draws BOTH split-pane dividers and the frame around the plugin
  overlay. `false` removes both (author confirmed the overlay "ring"
  disappears); no thin-line or overlay-only middle ground, `[theme]
  accent` only colors the border when on. Recommend pane_borders=false
  only for one-pane-per-tab layouts.
- placement: a plugin overlay renders FULL-WINDOW; `--placement overlay`
  and `zoomed` look identical (author TTY-tested). No pane-scoped
  floating overlay in 0.7.1 -> "open inside the current pane only" is an
  upstream gap, same family as in-place (floating/popup surfaces #785,
  #812). `split`/`tab` open a separate pane/tab.
- archive table: no trigger. Reinforces the in-place item's priority
  (author dislikes the framed/full-window overlay; wants true in-place).

## 2026-07-07 (herdr v0.7.1 installed, v0.7.1 latest stable)

Baseline entry from the initial research pass (full details in
docs/ROADMAP.md evidence appendix).

- changelog: copy mode exists (#231); no native search anywhere.
- discussion #563 ("Add / and ? search in copy mode"): open,
  unanswered, no maintainer commitment, no planned label.
- copy-mode API sentence: PRESENT ("No API exists for extending or
  overriding copy mode in v1", https://herdr.dev/docs/plugins/).
- competitors: rmarganti/herdr-pluck = visible-screen hints only; no
  scrollback search plugin found across 111 herdr-plugin repos.
- empirical contract (verified 2026-07-06/07, herdr 0.7.1): pane read
  cap ~1000 lines (999 recent-unwrapped); OSC 52 needs ~200 ms exit
  grace; overlay covers focused pane, restores focus+zoom; details in
  CLAUDE.md "herdr plugin contract".
- archive table: no trigger.
- open questions for next run: does `pane read --source detection`
  expose prompt/command marks (would unblock roadmap F7)? Is there a
  plugin-facing clipboard API that could replace raw OSC 52 writes to
  /dev/tty (would remove the 200 ms sleep and the ~100 KB payload
  truncation risk)? Did #481's socket auth scoping leave pane.read
  unrestricted?
