# Homing

Homing is a Windows-only Rust + Tauri tray app. Public source, issues, commits, and documentation are English. Keep `README_zh-Hans.md` synchronized with the authoritative English README.

## Safety and behavior

- Read `docs/features/wifi-reconnect.md` before changing reconnect, settings, or flyout behavior.
- Put all network operations behind `WifiBackend`. Use simulated tests; CI has no Wi-Fi hardware.
- Never disconnect, reconnect, change Wi-Fi, or launch with an existing configuration on a machine carrying remote sessions. Read-only queries and browser mock preview are safe.
- Preserve the predecessor's case-sensitive literal substring selection and one-second/61-second retry timing. Keep callbacks non-blocking; network work belongs to the worker.
- Startup belongs to external management. Keep Tauri's initial window list empty; destroy the flyout on dismissal to release its webview.

## Delivery

The bootstrap ticket owns the initial uncommitted tree; its coordinator makes the signed root commit and publishes it. After bootstrap, GitHub Issues in `Hezric/Homing` is the tracker. Specifications are parent issues and tickets are native sub-issues. Labels: `ready-for-agent`, `in-progress`, `blocked`, `done`. Close accepted work with its PR/commit and evidence.

Use short-lived `<type>/<short-desc>` branches and one squash PR per coherent change. Every commit and PR title follows scoped Conventional Commits in `cchk.toml`, lowercase where meaning permits. First commit: `chore(repo): initial commit`. Sign commits and release tags. If signing is unavailable, retain work locally and restore signatures before publishing.

GitHub CI is the test authority. Local runs are builds and simulated tests; the bootstrap coordinator runs CI after publishing. Install missing development tools through Scoop. Run the CI commands in `.github/workflows/ci.yml` before delivery. Required job names are `test` and `policy`. Main stays signed and linear, with strict current checks and no force pushes, deletion, or bypass actors. A direct update requires that exact signed SHA to have passed CI on another branch against current main. Rebase stale branches locally and re-sign; use `--force-with-lease` only on a development branch. Maintainers squash-merge with the PR title and body, verify GitHub's resulting signature, and delete merged branches.

Dependabot keeps native branch names; only its branch-name check is exempt. Weekly grouped updates require scoped messages, green checks, signatures, and squash merging. Add `.github/dependabot.yml` after bootstrap per the setup workflow; it is deliberately absent from the root tree.

Keep progress in Git and Issues, current behavior in `docs/features/`, canonical language in `GLOSSARY.md` only when needed, and consequential architectural rationale in `docs/adr/` only when a real decision needs preserving.

## Release

Release only on explicit user instruction. The single `Release due` issue is advisory: security/breaking change, five user-visible changes, or one after 30 days. Use git-cliff with `cliff.toml`, update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` together in `chore(release): vX.Y.Z`. After the tested squash merge, verify its signature and create a signed annotated immutable `vX.Y.Z` tag. The release workflow validates the source, builds portable archives with dependency notices, and publishes changelog notes and SHA256. Verify the downloaded artifacts and signatures before reporting a release.

