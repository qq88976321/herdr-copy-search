# CL-04: generate README demo GIFs with vhs

Context: demo/*.tape files script every keystroke of the demos, so
any agent can regenerate up-to-date GIFs after a UI change - no human
recording. The tapes were authored 2026-07-07 and are UNVERIFIED
until this checklist has been run once. The binary runs against
fixtures/sample.txt in a plain terminal (no herdr needed), which
makes the recording deterministic.

Prerequisites (pick ONE):
- docker: `docker run --rm -v "$PWD":/vhs ghcr.io/charmbracelet/vhs <tape>`
  (image bundles ttyd + ffmpeg). Needs network access to ghcr.io the
  first time.
- native: install vhs + ttyd + ffmpeg. vhs releases:
  https://github.com/charmbracelet/vhs/releases (single binary);
  ttyd: https://github.com/tsl0922/ttyd/releases; ffmpeg from the
  system package manager.

Steps:
1. [ ] `cargo build --release` (tapes call the built binary; building
       inside the recording would bloat it).
2. [ ] For each tape: `vhs demo/copy.tape` and `vhs demo/extract.tape`
       (or the docker equivalent). Expect demo/copy.gif and
       demo/extract.gif.
3. [ ] Acceptance criteria - open each GIF and check:
       - [ ] no shell prompt or build output visible (Hide/Show works)
       - [ ] copy.gif: fixture content visible, search prompt shows a
             typed query, matches highlighted, a selection is made,
             the [copy]/[search] mode row is visible at the bottom
       - [ ] extract.gif: bottom popup over the pane content, fuzzy
             query narrows the list, a pick is highlighted
       - [ ] no truncated final frame (the quit keystroke rendered)
       - [ ] file size: each GIF under 2 MB. If larger: reduce
             Set Width/Height, raise TypingSpeed, or shorten Sleeps
             in the tape; regenerate.
       If a criterion fails, edit the tape (timing is the usual
       culprit: prefer `Wait+Screen /regex/` over `Sleep` where
       possible), regenerate, re-check.
4. [ ] First successful run: remove the "UNVERIFIED" note from the
       tape file headers.
5. [ ] Embed in README (after the intro paragraph):
       `![copy mode demo](demo/copy.gif)` and in the Extract section
       `![extract demo](demo/extract.gif)`.
6. [ ] Commit GIFs + tape/README edits together:
       `docs(demo): record copy and extract demos`.
       GIFs are binary but small; committing them to the repo is the
       normal pattern (ratatui does the same). Do not use git-lfs.

Regeneration rule (add to your dev loop): any commit that changes
rendering, keybindings, or the fixture MUST regenerate the GIFs in
the same branch (rerun steps 1-3, 6). If drift is found later, treat
it as a bug: regenerate immediately.

Optional CI (ask the user first): charmbracelet/vhs-action@v2 can
regenerate GIFs on demand in GitHub Actions. Prefer keeping it a
manual/agent-run step: a render-diff in every PR is noise for a
single-user repo.

Done criteria: both GIFs in repo, embedded in README, criteria pass,
tapes no longer marked UNVERIFIED.
