# Legohan's Client

An independent implementation of the Minecraft: Java Edition **1.21.11** client, written entirely in Rust.

> **Status: Task 001 (workspace skeleton).** Nothing playable exists yet. `lc-app` only prints version information.

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
| `lc-app` | The `lc-app` binary that wires everything together |
| `xtask` | Developer tooling, run with `cargo xtask` |

Dependencies point one way only:

```
lc-app -> lc-ui / lc-render / lc-input / lc-audio -> lc-world -> lc-protocol / lc-registry
lc-net -> lc-protocol
```

`lc-protocol` and `lc-registry` must never depend on graphics, audio or OS-specific crates.

## Building

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.91.0). With [rustup](https://rustup.rs) installed:

```
cargo run -p lc-app -- --version
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Continuous integration

`.github/workflows/ci.yml` runs on every push and pull request: formatting, clippy (warnings are errors) and tests, on Windows, Linux and macOS.

## License

Not yet chosen.
