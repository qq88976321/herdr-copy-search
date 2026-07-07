# CL-07: upstream watch (herdr) and archive decision

Run this: quarterly; before starting any Phase 2 roadmap item; or
after updating herdr past a minor version. Takes ~15 minutes. Record
every run in docs/upstream-log.md (append a dated entry, template at
the bottom of that file) - even "no change" runs, so staleness is
visible.

## Collect facts

1. [ ] Installed vs latest herdr:
       `herdr --version`
       `curl -s https://api.github.com/repos/ogulcancelik/herdr/releases/latest | grep tag_name`
2. [ ] Changelog scan since the last logged version:
       https://github.com/ogulcancelik/herdr/blob/main/CHANGELOG.md
       Search for: "copy mode", "search", "plugin", "pane read",
       "clipboard", "OSC". Note anything relevant.
3. [ ] Native search demand thread still open/unanswered?
       https://github.com/ogulcancelik/herdr/discussions/563
4. [ ] Copy-mode plugin API: fetch https://herdr.dev/docs/plugins/
       and check whether the sentence "No API exists for extending
       or overriding copy mode" is still present. Its disappearance
       is trigger A2 in the roadmap archive table.
5. [ ] Competitor check: any plugin doing scrollback SEARCH (not just
       visible-screen hints)?
       - https://github.com/rmarganti/herdr-pluck (recent releases?)
       - https://github.com/yigitkonur/awesome-herdr (new entries in
         copy/search category)
       - https://github.com/topics/herdr-plugin sorted by recently
         updated
6. [ ] Tracked upstream issues - state changes?
       #680 (copy mode follows live output), #970 (vim word
       boundaries), #785/#812 (floating plugin surfaces), #893
       (plugin registry clobber), #978 (--lines 0 empty).

## Re-verify empirical contract (only if herdr version changed)

These facts are load-bearing, undocumented upstream, and may silently
change (they live in CLAUDE.md "herdr plugin contract"):
7. [ ] pane read line cap: in a pane, `seq 3000`, then
       `herdr pane read --pane <id> --lines 3000 --format text | wc -l`
       Expect ~1000. If it changed, update CLAUDE.md and consider the
       buffer.rs implications (larger reads = perf check).
8. [ ] OSC 52 grace period still needed: temporarily lower the sleep
       in osc52.rs to 0 in a scratch build, yank inside herdr, check
       whether the clipboard still receives it. If herdr now flushes
       synchronously, the 200 ms sleep can go (roadmap note).
9. [ ] Overlay behavior: does the overlay still cover only the
       focused pane and restore focus+zoom on close?

## Decide

10.[ ] Walk the Archive conditions table in docs/ROADMAP.md (A1-A5)
       against the facts above. Outcomes:
       - no trigger: log "no change", done
       - trigger A1/A2/A3: STOP, present the evidence to the user
         with a recommendation (archive / port / strip down). Never
         archive without the user's explicit decision.
       - trigger A4: pin the last working herdr version in README,
         tell the user
11.[ ] New herdr features that UNBLOCK roadmap items (e.g. OSC 133 /
       prompt marks exposure for F7, popup surfaces for extract):
       update the item's status in docs/ROADMAP.md from [!] and tell
       the user it is now buildable.
12.[ ] Append the dated entry to docs/upstream-log.md, commit:
       `docs(upstream): watch run YYYY-MM-DD`.
