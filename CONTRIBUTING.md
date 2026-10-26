# Contributing to Duskytunes

Duskytunes is a Dusky-first desktop application, so changes are reviewed for behaviour, integration and steady-state cost — not only whether they compile.

## Start here

For development setup, use [docs/INSTALL-ARCH.md](docs/INSTALL-ARCH.md).

Frontend:

```bash
cd ui
pnpm install --frozen-lockfile
pnpm check
pnpm build
```

Native:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo check --workspace --locked
```

For local development with the desktop host:

```bash
cargo tauri dev
```

## Preserve the product contracts

Changes must not regress these release-critical behaviours:

- audio-only native playback through the backend/libmpv;
- background WebKit hibernation while native playback and MPRIS continue;
- tray-only playback staying alive;
- tray-only idle exit when nothing is playing;
- explicit Quit fully stopping playback and unregistering media state;
- event-driven playback without a permanent high-frequency frontend transport timer;
- stable Home DOM without physical section virtualization;
- independent `Duskytunes Mini` geometry;
- replacement packaging that never removes `dusky-desktop`.

## UI conventions

- Treat Dusky live semantic tokens as the Follow System source of truth.
- Do not create an independent palette layer that only partly follows Dusky.
- Artwork may be colourful; functional chrome stays restrained.
- Prefer one-pixel structure, compact geometry and existing UI primitives.
- Use existing iconography before introducing another library.
- Motion should communicate state, not run as ambient decoration.
- Honour reduced-motion and Dusky motion settings.
- Do not add custom wheel physics.
- Keep network/provider logic behind the Rust command boundary.

See [docs/DESIGN.md](docs/DESIGN.md) for the visual language.

## Performance review

If a change touches playback UI, lyrics, artwork, Home, visualizers, WebViews or timers, include a quick steady-state check.

Look for:

- new intervals/timeouts that repeat forever;
- requestAnimationFrame loops;
- hidden WebKit work;
- repeated image decoding;
- unnecessary compositor animation;
- work that continues after the main WebView is hibernated.

## Pull requests

Keep PRs concrete:

1. what changed;
2. why it changed;
3. what lifecycle or UI surface is affected;
4. how it was validated;
5. screenshots or measurements when they improve review quality.

For large architectural changes, open an issue first so the constraints can be agreed before implementation.

Provider/network features should fail softly. An external service going down must not make local playback or the desktop shell unusable.
