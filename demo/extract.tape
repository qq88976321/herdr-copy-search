# vhs tape: extract mode demo for the README.
# Regenerate: see docs/checklists/CL-04-demo-recording.md (docker vhs
# with a CJK font mount so the fixture's wide chars render). Produces
# demo/extract.mp4 at 2x (2400x1400) for crisp text.
#
# Flow: open the extract picker on the fixture, fuzzy-filter tokens,
# move the selection, toggle line granularity, copy the pick (Tab).

Output demo/extract.mp4

Set FontSize 32
Set Width 2400
Set Height 1400
Set Padding 32
Set TypingSpeed 100ms
# 12fps capture: at 2x the headless screenshots are slow, so higher
# rates drop frames and the clip plays too fast (playback stays 25fps).
Set Framerate 12
# Play back 20% slower for readability.
Set PlaybackSpeed 0.8

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
