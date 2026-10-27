## What changed

<!-- Concrete behaviour, architecture, UI or packaging changes. -->

## Why

<!-- What user problem, regression or maintenance issue does this solve? -->

## Surface area

<!-- Home / Search / Library / playback / mini-player / theming / lifecycle / packaging / provider / docs -->

## Validation

- [ ] `./scripts/release-check.sh`
- [ ] `cd ui && pnpm check && pnpm build`
- [ ] `cargo fmt --all -- --check`
- [ ] relevant Rust/frontend tests
- [ ] affected Dusky/Hyprland lifecycle tested when applicable

### Regression contracts

Check any that this PR touches:

- [ ] background WebKit hibernation
- [ ] MPRIS / tray / explicit Quit semantics
- [ ] stable Home DOM / scrolling
- [ ] live Dusky theme bridge
- [ ] main vs mini-player window rules
- [ ] replacement-package rollback / `dusky-desktop` safety
- [ ] steady-state CPU / frontend timer behaviour

## Evidence

<!-- Screenshots, logs, performance measurements, package ownership output, or migration notes. -->

## Follow-up

<!-- Optional: intentionally deferred work. -->
