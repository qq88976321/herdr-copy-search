# vhs tape: copy/search mode demo for the README.
# Regenerate: see docs/checklists/CL-04-demo-recording.md (docker vhs
# with a CJK font mount so the fixture's wide chars render). Produces
# demo/copy.mp4 at 2x (2400x1400) for crisp text.
#
# Flow: open copy mode on the fixture, search up for "error"
# incrementally, walk matches with n, then pattern-select a URL from
# the p menu and yank it.

Output demo/copy.mp4

Set FontSize 32
Set Width 2400
Set Height 1400
Set Padding 32
Set TypingSpeed 100ms
# 12fps capture: at 2x the headless screenshots are slow, so higher
# rates drop frames and the clip plays too fast (playback stays 25fps).
Set Framerate 12

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
