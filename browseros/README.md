# DeckCraft on BrowserOS

This branch builds DeckCraft for [BrowserOS](https://github.com/Progate/packages/tree/main/packages/browser-os)
(`wasm32-wasip1-threads`). The window, input, CPU painting and the file dialogs live in `@progate/browser-deck`
(on top of `packages/browser-egui`); this fork only holds what the DeckCraft crates need to build and run there.

## Changes

- **`cfg(target_arch = "wasm32")` means the web build.** Every such `cfg` in `crates/` and `apps/` became
  `cfg(all(target_arch = "wasm32", target_os = "unknown"))`. The web build (wasm-bindgen, `js-sys`, `web-sys`) stays
  as it was; on WASI the crates take the native path (`std::fs`, `std::time`, threads), as they do on Linux. The
  `system_font_dirs` `cfg!` arm that meant "the browser has no fonts" is narrowed the same way, so WASI reads
  `/usr/share/fonts` like Linux does.
- **WASI dropped-file read** (`crates/ui-egui/src/lib.rs`): egui's wasm `DroppedFile` exposes no `bytes()`, so the
  non-web arm that used it split in two — WASI reads the path with `std::fs::read`, native keeps `bytes()`, the
  browser keeps the inbox path.
- **`parking_lot_core` waits with std's park / unpark on WASI** (`browseros/crates/parking_lot_core`, patched in
  through `[patch.crates-io]`). Upstream's wasm thread parker panics as soon as a lock is contended unless the
  nightly feature is on; the std of `wasm32-wasip1-threads` implements park / unpark with
  `memory.atomic.wait32` / `notify`, so `thread_parker/wasi.rs` waits on that. The rest is crates.io's 0.9.12.

## For the Progate PowerPoint course (`nakedtatsuya/office-course`)

- **Japanese interface** (`crates/ui-egui/src/i18n`, WordCraft's catalog system): `DECKCRAFT_LANGUAGE`, then
  `LC_ALL` / `LC_MESSAGES` / `LANG` pick the language; English stays the default. Labels use Microsoft PowerPoint
  (Microsoft 365) Japanese feature and menu names so MOS-style instructions match the screen. The catalog is
  `crates/ui-egui/src/i18n/ja.tsv`, the terminology table `docs/localization-ja.md`. Empty placeholders show their
  prompt in the interface language through `deckcraft_render::set_prompt_text` (never written to files).
- **Chart Design › Add Chart Element / Quick Layout** (`chart.options` gains `minorGridlines`, `xAxisTitle`,
  `yAxisTitle`; new `chart.quickLayout`).
- **Media playback options are saved**: `p:timing` gets `p:video`/`p:audio` > `p:cMediaNode` (`vol`, `mute`,
  `numSld`, `showWhenStopped`) > `p:cTn` (`repeatCount="indefinite"`, `fill="remove"`), `fullScrn` on `p:video`,
  and "Start: Automatically" as a `mediacall` Play effect before the first click; all read back.
- **File › Info › Check for Issues › Inspect Document** (`file.inspect`, `file.inspectRemove`): comments, document
  properties and personal information, hidden slides, off-slide content, presentation notes.
- **Mark as Final** (`_MarkAsFinal` in `docProps/custom.xml`, `file.markFinal` / `file.editAnyway`) and the
  **password to modify** (`p:modifyVerifier`, SHA-512 with salt and 100000 spins, `file.setModifyPassword` /
  `file.unlock`). Opening such a file locks editing (`DocState::lock`) until Edit Anyway, the password, or Read Only.

## License

As the rest of DeckCraft (MIT OR Apache-2.0). `browseros/crates/parking_lot_core` keeps its own license
(MIT OR Apache-2.0, see its `LICENSE-*`).
