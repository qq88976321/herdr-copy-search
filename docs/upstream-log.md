# Upstream watch log

Append-only log of CL-07 runs (newest first). Entry template:

    ## YYYY-MM-DD (herdr vX.Y.Z installed, vX.Y.Z latest)
    - changelog: <relevant items or "nothing relevant">
    - discussion #563: <open/answered/closed>
    - copy-mode API sentence: <present/absent>
    - competitors: <changes or "none">
    - empirical contract: <re-verified/skipped (no version change)>
    - archive table: <no trigger / trigger Ax -> action taken>

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
