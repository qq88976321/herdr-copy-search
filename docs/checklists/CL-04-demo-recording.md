# CL-04: generate README demo videos with vhs

Context: demo/*.tape files script every keystroke of the demos, so
any agent can regenerate up-to-date videos after a UI change - no human
recording. The binary runs against fixtures/sample.txt in a plain
terminal (no herdr needed), which makes the recording deterministic.
Output is mp4 at 2x (2400x1400) for crisp text; the fixture has a
Traditional-Chinese wide-char line, so the render needs a CJK font.

Prerequisites: docker (this host has no native vhs/ttyd/ffmpeg). The
`ghcr.io/charmbracelet/vhs` image bundles ttyd + ffmpeg but ships only
Latin fonts, so a CJK font is mounted from the host at render time.
Verify one exists: `fc-list :lang=zh` (this host has Noto Sans CJK at
/usr/share/fonts/opentype/noto). The image pull and docker.sock need
the command sandbox OFF.

Steps:
1. [ ] `cargo build --release` on the host (the tape's inner
       `cargo build` no-ops inside the image - no rust there - and runs
       the mounted ./target/release binary).
2. [ ] Render each tape. `--user` keeps the output owned by you (else
       root); `-e HOME=/tmp` gives fontconfig/chromium a writable home;
       the noto mount + `fc-cache` let chromium fall back to CJK glyphs:
       ```
       docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp \
         -v "$PWD":/vhs \
         -v /usr/share/fonts/opentype/noto:/usr/share/fonts/noto-cjk:ro \
         --entrypoint sh ghcr.io/charmbracelet/vhs \
         -c 'fc-cache -f >/dev/null 2>&1; vhs demo/extract.tape'
       ```
       Repeat for demo/copy.tape. Expect demo/copy.mp4 and
       demo/extract.mp4.
3. [ ] Acceptance criteria - inspect each mp4 (grab frames with the
       image's ffmpeg: `ffmpeg -ss <t> -i demo/x.mp4 -frames:v 1 f.png`,
       and check duration with `ffprobe -show_entries format=duration`):
       - [ ] no shell prompt or build output visible (Hide/Show works)
       - [ ] duration matches the tape's Sleeps (extract ~8s, copy ~10s).
             If it is a few seconds long / plays too fast, the 2x capture
             is dropping frames: LOWER `Set Framerate` (12 works; the
             default is too high at 2x). Playback stays 25fps regardless.
       - [ ] CJK line renders as glyphs, not boxes (copy.mp4 opening
             screen: "wide: <chinese> CJK width test ..."). Boxes mean
             the noto mount / fc-cache step was skipped.
       - [ ] copy.mp4: search prompt shows a typed query, matches
             highlighted, a selection is made, the [copy]/[search] mode
             row is visible at the bottom
       - [ ] extract.mp4: candidates -> hint -> separator -> input top to
             bottom, a divider above the candidates, the [word]/[line]
             badge colored like [copy]/[search]
       - [ ] no truncated final frame (the quit keystroke rendered)
4. [ ] Embed in README: a repo-relative `<video>` src does NOT play
       inline on github.com. Keep the `<video src="demo/copy.mp4">` tag
       for local/other renderers; for an inline player on GitHub, after
       pushing, drag the mp4 into the README/issue editor on the web to
       get a user-attachments URL and use that as the src (see the
       HTML comment in README). GitHub video-upload docs:
       https://github.blog/news-insights/product-news/video-uploads-available-github/
5. [ ] Commit videos + tape/README edits together:
       `docs(demo): record copy and extract demos`.
       mp4s are binary but small (< 1 MB); committing them to the repo
       is fine. Do not use git-lfs.

Regeneration rule (add to your dev loop): any commit that changes
rendering, keybindings, or the fixture MUST regenerate the videos in
the same branch (rerun steps 1-3, 5). If drift is found later, treat
it as a bug: regenerate immediately.

Optional CI (ask the user first): charmbracelet/vhs-action@v2 can
regenerate on demand in GitHub Actions, but it would need the CJK font
installed in the action and a render-diff in every PR is noise for a
single-user repo. Prefer keeping it a manual/agent-run step.

Done criteria: both mp4s in repo, referenced in README, criteria pass.
