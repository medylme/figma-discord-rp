# Figma Rich Presence

Shows your active Figma file as a Discord Rich Presence activity. Runs as a system tray app.

## Requirements

### Runtime

- [Discord](https://discord.com) desktop app running
- [Figma](https://figma.com) open in a browser or desktop app

**Linux only** — the following system libraries must be installed:

```bash
sudo apt install -y libgtk-3-0 libayatana-appindicator3-1
```

### Building

- [Rust](https://rustup.rs) toolchain (stable)
- [just](https://github.com/casey/just) task runner

**Linux** — also requires:

```bash
sudo apt install -y libglib2.0-dev libgtk-3-dev libayatana-appindicator3-dev pkg-config
```

## Building from source

Create a `.env` file with your Discord application ID:

```
DISCORD_APP_ID=your_app_id_here
```

Then run the application:

```
just dev
```

Or build for the current platform:

```
just build
```

## Cross-compiling

Release binaries for each supported platform are built with the `just dist-*` commands.
Not all targets can be built from every host.

| Host platform | Supported targets |
| --- | --- |
| Linux / WSL | Linux, Windows |
| macOS | macOS (x86 + ARM) |

**Linux / WSL** — requires the Linux dev packages above, plus MinGW for Windows:

```bash
sudo apt install -y gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
```

**macOS** — add whichever target you don't have natively:

```bash
rustup target add x86_64-apple-darwin    # Intel
rustup target add aarch64-apple-darwin   # Apple Silicon
```

Then build release binaries:

```
just dist-win      # Windows (from Linux / WSL)
just dist-linux    # Linux   (from Linux / WSL)
just dist-mac      # macOS   (from macOS)
```

The following env vars can optionally be set (or added to `.env`):

```
DIST_DIR=/dist       # where release binaries are copied (default: dist)
TARGET_DIR=/target   # cargo target directory (default: target)
```

## Settings

Right-click the tray icon and open **Settings** to configure:

- App name shown in Discord (Figma, Figma Desktop, or a custom name)
- Hide file names (Privacy Mode)
- Disable idle detection
- Per-state image URL overrides

Settings are saved to your OS config directory.
