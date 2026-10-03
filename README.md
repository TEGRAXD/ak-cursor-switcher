# Arknights Cursor Switcher (Windows)

[![Version](https://img.shields.io/badge/version-0.2.0-blue)](https://github.com/TEGRAXD/ak-cursor-switcher)

Lightweight native Windows utility to disable PRTS software cursor, fix mouse stuttering during loading screens, and auto-scale custom cursor schemes for Arknights PC Client, written in Rust with zero heavy GUI dependencies.

## Features

- **Disable PRTS Cursor**: One-click toggle (`.bin` ↔ `.bin.disabled`) to eliminate loading screen mouse stutter.
- **Auto-Switch**: Applies chosen cursor scheme & size when `Arknights.exe` runs, restores defaults on exit.
- **Live Scaling**: Scale cursor from 32px to 112px (Size 1–6) instantly.
- **System Tray**: Background monitoring with quick-switch tray menu.
- **Lightweight**: Pure Win32, no runtime dependencies.

## PRTS Cursor

The official Arknights PC client loads a custom Unity PRTS software cursor bundle:
```
Arknights_Data\StreamingAssets\AB\Windows\anon\a9d41799f1af1868f2db495671227cd4.bin
```

Because software cursors render inside the game loop, mouse movement freezes and stutters heavily whenever the game loads assets, switches scenes, or drops frames.

### Disclaimer

- **Client-Side Asset Toggle**: Simply renames an asset bundle on disk so Windows renders its native cursor. No memory injection, hooks, or executable modifications.
- **Direct Launch**: Run `Arknights.exe` directly (or via desktop shortcut) rather than the official launcher. The launcher's startup file verification may detect the renamed file and attempt to re-download it.

## Configuration

Settings are stored next to the executable in `settings.ini`:

```ini
target_process=Arknights.exe
scheme=Windows Aero
size=2
auto_switch=true
game_path=D:\YostarGames\Arknights\Arknights_Data\StreamingAssets\AB\Windows\anon
```

## Build & Run

### Prerequisites
- Windows 10 / 11
- Rust toolchain (`msvc` target recommended)

### Build Command

```sh
cargo build --release
```

Binary outputs to `target/release/ak-cursor-switcher.exe`.

## License
```
Apache-2.0 License
```
