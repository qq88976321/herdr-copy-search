# Upstream watch log

Append-only log of CL-07 runs (newest first). Entry template:

    ## YYYY-MM-DD (herdr vX.Y.Z installed, vX.Y.Z latest)
    - changelog: <relevant items or "nothing relevant">
    - discussion #563: <open/answered/closed>
    - copy-mode API sentence: <present/absent>
    - competitors: <changes or "none">
    - empirical contract: <re-verified/skipped (no version change)>
    - archive table: <no trigger / trigger Ax -> action taken>

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
