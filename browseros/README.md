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

## License

As the rest of DeckCraft (MIT OR Apache-2.0). `browseros/crates/parking_lot_core` keeps its own license
(MIT OR Apache-2.0, see its `LICENSE-*`).
