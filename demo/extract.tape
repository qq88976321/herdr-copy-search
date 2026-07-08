# vhs tape: extract mode demo for the README.
# Regenerate with: vhs demo/extract.tape  (needs vhs + ttyd + ffmpeg, or
# docker run --rm -v "$PWD":/vhs ghcr.io/charmbracelet/vhs demo/extract.tape)
# Procedure and acceptance criteria: docs/checklists/CL-04-demo-recording.md
#
# Flow: open the extract picker on the fixture, fuzzy-filter tokens,
# move the selection, toggle line granularity, copy the pick (Tab).

Output demo/extract.gif

Set FontSize 16
Set Width 1200
Set Height 700
Set Padding 16
Set TypingSpeed 100ms

Hide
Type "cargo build --release --quiet 2>/dev/null; clear; ./target/release/herdr-copy-search --input fixtures/sample.txt --mode extract"
Enter
Wait+Screen@60s /\[word\]/
Show

Sleep 1.5s

# fuzzy-filter down to URL-ish tokens
Type "http"
Sleep 1.2s
Down
Sleep 600ms
Down
Sleep 1s

# toggle word -> line granularity and back
Ctrl+t
Sleep 1.2s
Ctrl+t
Sleep 1s

# copy the pick via OSC 52 (exits)
Tab
Sleep 1.5s

Hide
