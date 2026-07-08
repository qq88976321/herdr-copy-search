# vhs tape: copy/search mode demo for the README.
# Regenerate with: vhs demo/copy.tape  (needs vhs + ttyd + ffmpeg, or
# docker run --rm -v "$PWD":/vhs ghcr.io/charmbracelet/vhs demo/copy.tape)
# Procedure and acceptance criteria: docs/checklists/CL-04-demo-recording.md
#
# Flow: open copy mode on the fixture, search up for "error"
# incrementally, walk matches with n, then pattern-select a URL from
# the p menu and yank it.

Output demo/copy.gif

Set FontSize 16
Set Width 1200
Set Height 700
Set Padding 16
Set TypingSpeed 100ms

Hide
Type "cargo build --release --quiet 2>/dev/null; clear; ./target/release/herdr-copy-search --input fixtures/sample.txt --mode copy"
Enter
Wait+Screen@60s /\[copy\]/
Show

Sleep 1.5s

# incremental up-search: matches highlight while typing
Type "?"
Sleep 500ms
Type "error"
Sleep 1s
Enter
Sleep 1s
Type "n"
Sleep 800ms
Type "n"
Sleep 1s

# pattern menu: select the nearest URL above the cursor, then yank it
Type "p"
Sleep 1.2s
Type "u"
Sleep 1.2s
Type "n"
Sleep 1s
Type "y"
Sleep 1.5s

Hide
Type "q"
