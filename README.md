# Figma Rich Presence

Shows your active Figma file as a Discord Rich Presence activity. Runs as a system tray app.

## Requirements

### Running

- [Discord](https://discord.com)
- [Figma Desktop](https://www.figma.com/downloads/)

**Linux only** - the following system libraries must be installed:

```bash
sudo apt install -y libgtk-3-0 libayatana-appindicator3-1
```

### Building

- [Rust](https://rustup.rs) toolchain (stable)
- [just](https://github.com/casey/just)

**Linux** - also requires:

```bash
sudo apt install -y libglib2.0-dev libgtk-3-dev libayatana-appindicator3-dev pkg-config
```

## Building from source

Create a `.env` file with your [Discord application](https://discord.com/developers/applications) ID:

```
DISCORD_APP_ID=your_app_id_here
GITHUB_LATEST_RELEASE_URL=https://api.github.com/repos/.../releases/latest    # optional, for auto-updater
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

For **Linux / WSL** - requires some dev packages:

```bash
sudo apt install -y gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
```

For **macOS** - add whichever target isn't available natively:

```bash
rustup target add x86_64-apple-darwin    # Intel
rustup target add aarch64-apple-darwin   # Apple Silicon
```

Then build release binaries:

```
just dist-win      # Windows (only Linux / WSL)
just dist-linux    # Linux   (only Linux / WSL)
just dist-mac      # macOS   (only macOS)
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
