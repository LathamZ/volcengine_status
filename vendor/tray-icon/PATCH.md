# Vendored `tray-icon` — local macOS 27 patch

**Upstream crate:** `tray-icon` **0.24.1** (MIT OR Apache-2.0, © Tauri Programme within The Commons Conservancy)
**Local change:** upstream PR [tauri-apps/tray-icon#365](https://github.com/tauri-apps/tray-icon/pull/365) — "fix(macos): restore tray click event forwarding on macOS 27" (merged 2026-09-16 as `42eb44ea1507d51b68a8b2fbb0d96a9c85f5b4cd`).

The Rust source is byte-identical to upstream 0.24.1 with only that PR applied.
`Cargo.toml` carries one extra, clearly marked `[lints.rust]` block: vendored path
dependencies do not get Cargo's `--cap-lints allow`, so without it upstream's
pre-existing `unused_unsafe` warnings appear on every local build.

To re-derive the diff against pristine 0.24.1:

```bash
diff -u ~/.cargo/registry/src/index.crates.io-*/tray-icon-0.24.1/src/platform_impl/macos/mod.rs \
        vendor/tray-icon/src/platform_impl/macos/mod.rs
```

## Why this exists

On macOS 27, when an `NSMenu` is attached to the `NSStatusItem`, AppKit consumes the
**left** click itself before it reaches the `TrayTarget` view that tray-icon installs on
top of the status button (`button.addSubview`, `src/platform_impl/macos/mod.rs`). The
result is that `TrayIconEvent::Click { button: Left, .. }` is never emitted — not even
the `Down` half — so `show_menu_on_left_click(false)` becomes meaningless and a
left-click app like this one can no longer open its popover. Right-clicks and
`Enter`/`Move`/`Leave` tracking events are unaffected, which is what makes the failure
look asymmetric.

The fix keeps `NSStatusItem.menu` `nil` in steady state and attaches it only for the
duration of `performClick`, so nothing is attached to steal the click. It is
version-agnostic: it does not rely on AppKit's menu interception on any OS version.

## Why it is vendored rather than upgraded

The fixed release is `tray-icon` 0.25.x, which `tauri` 2.11.5 cannot use:

- tauri 2.11.5 requires `tray-icon = "0.24"` — caret on a `0.x` means `< 0.25.0`.
- tauri requests the feature `gtk`, renamed to `libappindicator` + `muda-gtk3` in 0.25.0.
- tauri re-exports `muda ^0.19` as `tauri::menu`, while tray-icon 0.25.x needs `muda ^0.20`,
  so the `Menu` tauri hands to `TrayIconBuilder::menu()` would be the wrong type.

(`tray-icon` 0.24.2 exists but is only a clippy refactor — it does **not** contain this fix.)

## Removing this

Delete `vendor/tray-icon/` and the `[patch.crates-io]` block in `src-tauri/Cargo.toml`
once tauri depends on `tray-icon >= 0.25` (tauri 3.x). If the patch is left in place
after such a bump, the build fails with a version conflict rather than silently
shipping unpatched code.
