# Legohan's Client

An independent implementation of the Minecraft: Java Edition **1.21.11** client, written entirely in Rust.

> **Status: Task 002 (launcher UI).** The launcher window works. The game itself does not exist yet, so nothing can be played.

## What works today and what does not

| Area | State |
|---|---|
| Launcher window, navigation, splash, themes, settings, persistence | Working |
| Installations (create, edit, duplicate, delete, select) | Working, saved to disk |
| Offline accounts (add, remove, switch) | Working, saved to disk |
| Servers (add, edit, remove, favorites, recently played) | Working |
| Server status (online, players, latency, MOTD, icon) | Working: a real server list ping over TCP. SRV records are not looked up |
| Resource packs (import, remove, preview, enable, order) | Working as storage. Packs are applied once the game exists |
| Screenshots (thumbnails, viewer, open folder) | Working for PNG files |
| Home artwork | Generated voxel landscape. Uses the panorama from your own Minecraft install if found (see below) |
| Update check | Placeholder: no update channel exists, and the splash says so |
| Microsoft sign-in | Not implemented. The button is disabled and no tokens are stored |
| Launching the game | Not possible: the Rust client is not implemented. Play explains this instead of pretending |

When a game backend implements `GameBackend` and reports `Ready`, Play launches it immediately with no extra steps.

### Artwork

Mojang's artwork is not redistributed. Home shows an original generated voxel landscape. If the official `1.21.11` client jar is found in your own `.minecraft` folder (or the folder set in Settings), its title-screen panorama is read from it at runtime instead. That loader is covered by tests using synthetic data and has not been run against a real Minecraft jar yet.

### Launcher data

Everything lives in one folder: `%APPDATA%\LegohansClient` on Windows, `~/Library/Application Support/LegohansClient` on macOS and `~/.local/share/legohans-client` on Linux. Set `LC_HOME` to use another folder. JSON files are written atomically, and an unreadable file is set aside as `*.corrupt-<time>.json` and reported instead of being overwritten.

Legohan's Client is not affiliated with, endorsed by, or associated with Mojang Studios or Microsoft.
Minecraft is a trademark of Mojang Studios.

## Principles

- **Independent:** protocol, world, rendering and UI are implemented here, not wrapped from another client.
- **No bundled game assets:** textures, sounds, language files and data reports are never committed or redistributed. They are read from the user's own Minecraft install or generated locally.
- **Authentication is never bypassed.**
  - Licensed play uses Microsoft account authentication (online mode).
  - Offline profiles are supported only for servers that themselves allow offline mode (`online-mode=false`).
  - Nothing in this project circumvents a server's authentication or security.
- **Windows first**, with Linux and macOS kept viable in the architecture.
- **Minimal dependencies:** third-party crates are added only when a task needs them.

## Workspace layout

| Crate | Responsibility |
|---|---|
| `lc-protocol` | Packet definitions, codecs, connection state machine |
| `lc-net` | TCP connection, compression, encryption, account kinds and authentication |
| `lc-registry` | Blocks, items, biomes, tags |
| `lc-world` | Chunks, palettes, lighting, entities, client-side state |
| `lc-render` | Meshing, texture atlas, GPU backend |
| `lc-audio` | Sound engine |
| `lc-input` | Keybinds, camera, movement and physics prediction |
| `lc-ui` | Menus, HUD, inventory screens |
| `lc-launcher` | The launcher window: pages, persistence, server pinging, game backend boundary |
| `lc-app` | Binaries: `lc-app` (command line, opens the launcher) and `legohans-client` (launcher without a console window on Windows) |
| `xtask` | Developer tooling, run with `cargo xtask` |

Dependencies point one way only:

```
lc-app -> lc-launcher -> lc-net -> lc-protocol
lc-app -> lc-ui / lc-render / lc-input / lc-audio -> lc-world -> lc-protocol / lc-registry
```

`lc-protocol` and `lc-registry` must never depend on graphics, audio or OS-specific crates.

## Building

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.91.0). With [rustup](https://rustup.rs) installed:

```
cargo run -p lc-app                 # opens the launcher
cargo run -p lc-app -- --version
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Continuous integration

`.github/workflows/ci.yml` runs on every push and pull request: formatting, clippy (warnings are errors), tests and a release build on Windows, Linux and macOS. The Windows job uploads `legohans-client.exe` and `lc-app.exe` as a downloadable artifact.

## License

Not yet chosen.
