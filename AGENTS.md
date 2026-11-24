# AGENTS.md · RPlayer

RPlayer is a Rust desktop music player built with `eframe/egui`. This file is
the contract for AI agents and human contributors. Keep it accurate.

## Build & test

Windows GNU toolchain on this machine:

```powershell
$env:RUSTUP_TOOLCHAIN='stable-x86_64-pc-windows-gnu'
$env:PATH='C:\Users\Randy\.cargo\bin;C:\Users\Randy\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin;' + $env:PATH
$env:CARGO_BUILD_JOBS='1'
cargo build --release --manifest-path D:\Workspace\RPlayer\Cargo.toml --bin rplayer
cargo test --release --manifest-path D:\Workspace\RPlayer\Cargo.toml
```

- Do not create `D:\Workspace\Cargo.toml`; the workspace root is `D:\Workspace\RPlayer`.
- Release profile uses `lto = "off"`, `codegen-units = 16` because this machine's
  commit limit cannot handle thin LTO with the media dependency tree.
- Version lives in `Cargo.toml`; the UI reads it with `env!("CARGO_PKG_VERSION")`
  and `build.rs` derives the exe `FileVersion`/`ProductVersion` from the same
  value. Do not hardcode a version string in source.

## Architecture

```
src/main.rs          eframe entry, viewport and renderer
src/app/             app state, views, panels, dialogs
  mod.rs             RPlayerApp, view switching, playback orchestration
  library_view.rs    main library/album/artist pages
  sidebar.rs         navigation and playlists
  now_playing.rs     full-screen lyrics page
  overlay.rs         desktop lyrics + mini player viewports
  settings.rs        settings window
src/audio.rs         rodio playback, gapless queue, EQ chain
src/playback.rs      CUE segment <-> whole-file position mapping
src/backup.rs        folder backups, validation and restore/rollback
src/model.rs         Library/Track/Playlist data types
src/cue.rs           CUE Sheet parsing and virtual track creation
src/duplicates.rs    duplicate group detection and grouping helpers
src/health.rs        missing-file and CUE relink helpers
src/tag_edit.rs      tag write and undo snapshots
src/storage.rs       local JSON settings + SQLite tracks
src/scanner.rs       background scanning and incremental reuse
src/session.rs       playback session persistence
src/lyrics.rs        LRC parsing and local/custom/cache loading
src/cover.rs         embedded -> sidecar -> cache cover priority
src/cover_fetch.rs   manual online cover lookup
src/lyrics_fetch.rs  manual LRCLIB lyrics lookup
src/platform.rs      Windows-specific window/SMTC/tray integration
src/theme.rs         palettes; all colors come from here
src/ui/widgets.rs    shared controls and animations
src/ui/motion.rs     motion tokens and reduced-motion switch
src/ui/focus.rs      2px focus rings for custom controls
```

## Invariants

- The app must work fully offline. `Library::offline_mode` defaults to `true`.
  Automatic lyric/cover network calls must stay behind this flag; manual
  actions may still use the network.
- All artist/title/album text must be rendered with egui; no fixed English UI.
- Track storage: `library.db` is authoritative for tracks; `library.json`
  stores settings, playlists, queue and favorites. Never drop one without a
  backup path.
- Write storage atomically: `.tmp` then rename, keep `.json.bak`.
- Do not reintroduce JSON as the only track store.
- Lyrics priority: user-edited custom cache -> sidecar `.lrc` -> embedded
  lyrics -> downloaded cache. Online cache is only a fallback.
- Cover priority: embedded -> sidecar -> cache -> manual online.
- `Track.modified` is Unix milliseconds and drives incremental scan reuse.
- UI code must use `theme::c()`; no hard-coded colors except color-key values
  in `platform.rs`.
- Every interactive control needs an AccessKit label/tooltip and a stable egui
  `Id` where practical. Automated verification uses these names.
- Avoid focus-stealing on the desktop lyrics window; use
  `platform::cursor_in_window` for hover.

## AI-friendly requirements (all future versions)

1. **Stable names** for views, buttons and settings. Do not rename Chinese UI
   labels without updating docs/tests.
2. **Structured code**: keep playback, storage, scanning, UI, platform separate.
3. **No hidden state**: new persisted fields need serde defaults and a
   migration note in `docs/`.
4. **Verifiable**: add a unit test for parsers/storage and list manual
   acceptance steps in the version plan.
5. **Docs**: update `README.md`, `docs/VX.Y_PLAN.md` and this file when
   architecture or commands change.
6. **Logs**: prefix platform warnings with `[RPlayer]`; never log secrets or
   full user file contents.
7. **Release**: `cargo test --release`, build release, update dist exe/docs,
   produce SHA-256 for the zip, commit all source/docs.

## UI conventions

- Spacing: 4/8/12/16 px; radius 10 for controls, 12 for cards, 18 for lyrics.
- Primary buttons use accent; secondary use card + border; danger uses danger.
- Page switches fade in via `view_fade` (0.18s ease-out). Keep animations
  subtle and respect reduced-motion if added later.
- Buttons scale to 0.975 while pressed via `animate_bool_with_time`.
- Custom controls draw focus with `src/ui/focus.rs`: `ring` for rounded
  rectangles, `ring_round` for circles, `ring_rect` for table rows/sliders.
- Text opacity and panel opacity are independent for desktop lyrics.

## Verification

- Unit tests: `cargo test --release` (currently 55+).
- Windows UI automation: `agent-desktop snapshot --window-id <id>`, then click
  by `ref_id`; prefer AccessKit names over pixel coordinates.
- Screenshots must be checked after UI changes; avoid leaving test images in
  `docs/` unless they are intentional documentation.
