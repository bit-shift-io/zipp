# Zipp (Rust + Flutter)

A high-performance, multiplatform Bluetooth application built with **Flutter** for the UI and **Rust** for the core logic.

> **Architecture Note**: This project uses **Rust** for all core application logic (UUIDs, parsing, state management) and **Dart/Flutter** strictly for the GUI and device transport. This separation allows for easily swapping the GUI framework in the future while preserving the business logic.

## Project Structure

### 🛠️ Core Logic (Rust)
The "brain" of the app lives in the `rust/` directory.
- **protocol.rs**: **Entry Point.** Contains the scooter protocol implementation.
  - `ScooterState`: The central data structure representing the bike (speed, battery, light status, etc.).
  - `parse_characteristic_data()`: Converts raw bytes from the bike into the `ScooterState`.
  - `create_command_bytes()`: Generates the raw bytes to send to the bike for actions like Power, Alarm, or Horn.
- **`src/api/`**: Contains the modules exposed to Flutter.

### 📱 Frontend (Flutter)
The UI and Bluetooth transport are in the `lib/` directory.
- **main.dart**: **GUI Entry Point.**
  - **Bluetooth Manager**: Handles scanning for "BScooter", auto-connecting, and characteristic subscriptions.
  - **Page Navigation**: Implements the fullscreen `PageView` (Dashboard <-> Log).
  - **Control Panel**: The button-based interface that triggers Rust commands.
- **`lib/src/rust/`**: **Generated Bridge.** Contains the Dart bindings that allow calling Rust functions seamlessly.

## Prerequisites

- **Flutter SDK**
- **Rust** (Stable)
- **Android NDK** (for Android builds)

## Setup & Running

### 1. Fix PATH & Install Tools (Once)
in the scripts repo run ``./install_android_flutter.sh``
OR the manual pain
install android + sdk + tools then:
```bash
sudo pacman -S cmake ninja
set -U fish_user_paths $HOME/.cargo/bin $fish_user_paths
cargo install flutter_rust_bridge_codegen
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
cargo install cargo-ndk
flutter doctor
```

### 2. Generate Code
Run this to link the Rust code and generate translations:
```bash
./build.sh
```
confirm adb on the device

### 3. Run the App
```bash
./run.sh
```

## App Icon

The source artwork is `assets/app_icon.svg`. After editing it, regenerate the
PNG source and platform icons from this directory (requires Inkscape):

```bash
inkscape assets/app_icon.svg --export-type=png --export-width=1024 --export-height=1024 --export-filename=assets/app_icon.png
dart run flutter_launcher_icons
for density in mdpi hdpi xhdpi xxhdpi xxxhdpi; do
  magick "android/app/src/main/res/mipmap-$density/mipmap.png" "android/app/src/main/res/mipmap-$density/ic_launcher.png"
done
```

The final step requires ImageMagick and refreshes Android's fallback icons as
well as the `mipmap` icons used by the app manifest.

## Features
- **Native Bluetooth**: Fast and reliable scanning using `flutter_blue_plus`.
- **HTTP Entry Control**: Control IoT entry points via HTTP requests (with offline queuing).
- **Log & Diagnostic Terminal**: Real-time logging of Bluetooth commands and responses with manual UART input.
- **Permission Handling**: Dynamic requests for Location and Bluetooth.
- **Dark Theme**: Modern Material 3 dark UI.

## Troubleshooting

- **Missing Imports**: If `main.dart` shows errors after generation, uncomment the bridge imports at the top of the file.
- **Cargo Expand**: If generation fails with "cargo expand returned empty output", run `cargo build` inside the `rust/` folder once.
