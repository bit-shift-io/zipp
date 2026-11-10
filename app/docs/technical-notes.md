# Zipp technical notes

Measured results from the port. Values that were never captured are marked
unknown rather than estimated.

## Environment

- Host: Fedora 44, kernel `7.2.11-200.fc44.x86_64`, x86_64; KDE Plasma
  **Wayland** session (`WAYLAND_DISPLAY=wayland-0`, `DISPLAY=:0`). The iced
  fork's `winit` is Wayland-only here.
- Rust: `rustc 1.98.1 (48a229cea 2026-09-01)` / `cargo 1.98.1 (797e8a9bc 2026-08-05)`;
  edition 2024.
- JDK: OpenJDK `21.0.10` at `/opt/jbr-21` (the build script auto-detects it).
- NDK: `28.2.13676358`; linker `aarch64-linux-android34-clang`, with
  `-C link-arg=-Wl,-z,max-page-size=16384` (16 KiB page support).
- Device / Android: Alldocube iPlay60 mini Turbo, Android 14 / SDK 34,
  1200×1920 @ 320 dpi, Adreno 710 (Vulkan), serial `6bfcc911`.

## Build measurements

- First `cargo check -p zipp`: **unknown** — no wall-clock timing was captured.
  The Task 12–17 reports only record that it finished clean with no warnings.
  (The cold cost is dominated by compiling the pinned iced / `iced_android`
  forks.)
- `scripts/build.sh debug`: Gradle `BUILD SUCCESSFUL in 44s`, Rust dev profile
  `Finished dev profile ... in 1m 06s` (first debug build on this host). APK:
  `app/build/app/outputs/apk/debug/app-debug.apk`, **40,608,181 bytes**, with
  `lib/arm64-v8a/libzipp.so`.
- Release APK size: **17,699,454 bytes** (~16.9 MiB),
  `app/build/app/outputs/apk/release/app-release.apk` (Rust release +
  Gradle `BUILD SUCCESSFUL in 17s`). Debug-key signed by design
  (`Signer #1 ... CN=Android Debug`); no `android/keystore.properties` exists,
  so it is not a distributable production build.

### Gate (Task 17)

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (was `-p zipp_core -p zipp` before Task 19 removed the Flutter crate) |
| `cargo clippy -p zipp --all-targets --target aarch64-linux-android -- -D warnings` | clean |
| `cargo test --workspace` | 72 passed (29 `zipp` + 43 `zipp_core`), 0 failed, 0 ignored |
| `cargo check -p zipp --target aarch64-linux-android` | clean |

## Device results

- BLE scan / auto-connect: **scan verified, auto-connect not verifiable**.
  `Scanning...` is shown with the stop icon and the LE scan really runs
  (`BtGatt.ScanManager startScan() [ScanClient ... io.bitshift.zipp]`, 15 s
  timeout then restart). No `BScooter` was in range, so finding it and
  auto-connecting could not be exercised.
- Notify / read / write: **not verifiable** — no `BScooter` in range. Covered
  by core unit tests and the desktop adapter compile only.
- GPS: **not possible on this hardware** — the tablet exposes no `gps`
  provider (only `passive`, `network`, `fused`), so the app takes the `disabled`
  branch and renders no GPS row. Not a defect.
- Wi-Fi / Entry: **not exercised**. The entry path is unit-tested and the
  desktop binary installs `env_logger` (so `log::info!`/`log::warn!` reach
  stdout) and Android writes the same lines to logcat, but no input-synthesis
  tool was available to click Entry and observe the network result.
- Rotation / background: **verified**. Rotation to landscape re-laid out the UI
  and kept the window; Home then relaunch brought the same process and
  `ActivityRecord` back (`LaunchState: HOT`, "its current task has been brought
  to the front") with no destroy/recreate and no errors.
- Logcat panic or JNI exception: **none**. A cold launch (`TotalTime 240–251 ms`)
  left the process alive; grepping the full buffer for `panic`,
  `FATAL EXCEPTION`, `JNI DETECTED`, `No implementation found`,
  `UnsatisfiedLinkError`, `SIGSEGV`, `SIGABRT` and ANR found nothing. The only
  notable line is the harmless
  `iced_android::android: Font not found or not usable: Roboto-Bold.ttf`, which
  falls back to another embedded face.

## Deviations

Accepted approximations against the original app's screens (all recorded in the
port ledger):

- Dashboard/log switch is an instant swap instead of the original 300 ms slide.
- Text baseline alignment is approximated with bottom alignment.
- The log-page and scanner dividers are `chrome::divider()` at **2 px**, where
  the original app used **1 px** in those two places (2 px on the dashboard in
  both).
- The scanner disconnect label is **14 px** vs the original app's **16 px**.
- The UART input placeholder (`...`) renders at the field's **18 px** rather
  than the original **20 px**; iced gives the value and the placeholder one
  size.

## Chinese font

- Detected font path (desktop): **none** — the explicit probe in `fonts.rs`
  (`cjk_font()`) checks a fixed list of Noto CJK / WQY / AR PL paths and
  none of them exists on this host, so the probe registers no CJK face.
- Caveat: iced's text stack (cosmic-text 0.15) also loads *all* system fonts
  through fontconfig (`db.load_system_fonts()`) and does per-glyph fallback.
  This host does have Noto Sans CJK, but only at
  `/usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc`, which the
  probe list does not cover, so Chinese glyphs are expected to resolve via that
  fallback. Chinese rendering was **not** visually confirmed on the desktop (the
  checklist ran in an English locale and no input tool was available).
- Android: `iced_android::fonts()` registers only Roboto, the emoji and symbol
  fonts and DroidSansMono — **no CJK face** — so `zipp`'s own probe registers
  `/system/fonts/NotoSansCJK-Regular.ttc` (present on the test device), falling
  back to `NotoSansCJKsc-Regular.otf` and DroidSansFallback.ttf.
