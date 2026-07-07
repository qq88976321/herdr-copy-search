# CL-03: release flow (local bump + GitHub artifacts)

Context: versioning is intentionally LOCAL-first. cargo-release bumps
Cargo.toml, syncs herdr-plugin.toml, commits, and tags (release.toml:
publish=false, push=false, pre-release-hook = just gate). The
.github/workflows/release.yml (authored 2026-07-07, UNVERIFIED until
the first tag push) builds binaries for a pushed v* tag and attaches
them to a GitHub Release. Release binaries are a convenience only:
`herdr plugin install` builds from source via herdr-plugin.toml
[[build]].

Rules:
- Releases are USER-ONLY: run this checklist only when the user asks
  for a release (CLAUDE.md rule).
- Never edit version fields by hand; cargo-release keeps Cargo.toml
  and herdr-plugin.toml in lockstep.

Steps:
1. [ ] Working tree clean, on master, CI green on HEAD.
2. [ ] `just release <patch|minor|major>` - runs the gate, bumps,
       commits, tags vX.Y.Z locally.
3. [ ] Push: `git push origin master && git push origin vX.Y.Z`.
4. [ ] `gh run list --workflow release.yml --limit 1` and watch it.
       Expect one job per target: x86_64-unknown-linux-gnu,
       x86_64-apple-darwin, aarch64-apple-darwin.
5. [ ] `gh release view vX.Y.Z` shows 3 archives + sha256 checksums.
6. [ ] First time only: when green, delete the "STATUS: UNVERIFIED"
       header comment from release.yml. Commit: `ci: mark release
       workflow verified`. If the action rejects an input (it is
       pinned by guess, never executed), read
       https://github.com/taiki-e/upload-rust-binary-action README
       and fix the input names; one commit.
7. [ ] Verify plugin update path: on a machine with the plugin
       installed from GitHub, `herdr plugin install` the new version
       (or the herdr-documented update command if one exists by now)
       and confirm the new version string in
       `herdr plugin list`.

Done criteria: tag on GitHub, release with 3 artifacts, plugin
updates cleanly.
