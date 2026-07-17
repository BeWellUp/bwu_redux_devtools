# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`bwu_redux_devtools` is the Redux DevTools GUI for `bwu_redux` (consumed from crates.io). It is both a binary (the DevTools desktop/web app) and a library (exposing the `redux` and `devtools_server` modules for reuse). It runs as a Dioxus app targeting desktop (GTK/WebKit) and web (WASM).

## Build & Run Commands

```bash
# Desktop (default feature)
dx serve --platform desktop

# Web (WASM) — must disable default features
dx serve --platform web --port 33333 --no-default-features --features web

# Standalone devtools gRPC server only (no GUI)
cargo run --bin bwu_redux_devtools_server --features standalone-server
```

`mprocs.yaml` defines named processes for running desktop, web, and Tailwind watchers together with `mprocs`.

## Development Workflow

```bash
cargo check
cargo check --target wasm32-unknown-unknown --no-default-features --features web
cargo test
cargo clippy --all-targets
cargo fmt        # rustfmt.toml uses unstable options; needs nightly rustfmt

# Tailwind CSS — must be running to pick up new utility classes
npm install      # once; installs daisyui and other CSS plugins
npm run css:watch
```

Note: do not build with `-Zcodegen-backend=cranelift` — the `pulp` crate (via
dioxus-desktop's image stack) fails to link under Cranelift.

## Feature Flags

| Feature | Purpose |
|---|---|
| `desktop` (default) | Desktop platform; enables file-based config, `redux_devtools_server` |
| `web` | WASM platform; enables `gloo-storage`, `tonic-web-wasm-client` |
| `redux_devtools` | Connects as a devtools client via gRPC-web WASM transport |
| `redux_devtools_server` | Embeds the Tonic gRPC devtools server (desktop only) |
| `standalone-server` | Builds the headless `bwu_redux_devtools_server` binary |
| `tokio-console` | tokio-console layer for the standalone server; needs `RUSTFLAGS="--cfg tokio_unstable"` |

## Architecture

### Library vs. Binary split

`src/lib.rs` exposes these public modules:
- `redux` — the Redux store, actions, reducers, state types, selectors, and middleware
- `devtools_server` (non-WASM only) — the embedded Tonic gRPC server
- `devtools_watch` (WASM only) — the gRPC-web `Watch` stream client feeding monitored apps' state changes into the store

`src/main.rs` is the Dioxus entry point. It wires together the store, the `DevtoolsServer` (desktop) or the `devtools_watch` client (web), and the Dioxus router.

### Components (`src/components/`)

`daisyui/` holds DaisyUI-styled components vendored from the bwu_app workspace's internal component crate: `drawer`, `menu`, and `kbd` (no official equivalent with the same look). `tabs/`, `virtual_list/`, and `tooltip/` are official Dioxus components copied in via `dx components add <name>`; their `style.css` files are restyled to use DaisyUI theme variables (`--color-*`, `--radius-field`) instead of the generated `dx-components-theme.css` palette (that file is deliberately not used — DaisyUI's own variables are the theme bridge, which keeps every component in sync with the theme switcher). Apply the same restyling pattern when adding further official components.

### Redux layer (`src/redux/`)

The app has its own Redux store (via `bwu_redux::StoreWrapper`) that tracks the state of *other* apps being monitored:

- **`State`**: top-level state holding `app_states: HashTrieMapSync<AppId, AppState>`, `selected_app_id`, theme, etc.
- **`AppState`**: per-monitored-app state holding a `history: QueueSync<StateChange>` (capped at 200 entries), `selected_state_id`, and `selected_state_viewer`.
- **`StateChange`**: a single recorded snapshot — `counter`, `session_counter`, serialized `action` (RON string), serialized `state` (RON string).
- **`StorageMiddleware`**: on desktop reads/writes theme preference to a TOML config file (`~/.config/bwu_redux_devtools/settings.toml`); on web uses `localStorage`.
- **`DevToolsActionFilter`**: prevents `Action::StateUpdate` from being forwarded to the devtools server to avoid recursive loops.
- Selectors in `src/redux/selectors.rs` expose `stream_*` functions that return `ChangesStream<T>` (a pinned async stream of distinct values from the store).

### DevTools server (`src/devtools_server/`)

On non-WASM builds, `DevtoolsServer` embeds a Tonic gRPC server listening on `[::1]:49051` (override with `BWU_REDUX_DEVTOOLS_ADDR`). It implements the `DevTools` service from `bwu_redux::devtools_rpc`:
- `state_change` — receives batched `StateChangeRequest` messages from monitored apps, records them in the `WatchHub`, and (when constructed with a dispatch sender, i.e. embedded in the desktop GUI) dispatches `Action::StateUpdate` into the local store.
- `connection_status` — health check.
- `watch` — server-streaming subscription for GUI clients: replays the buffered history (`replay = true`, up to 200 entries per app from `watch_hub.rs`), then streams live changes.

The standalone binary (`src/standalone_server.rs`) runs `DevtoolsServer::new(None)` — no local store; the `WatchHub` is the only buffer.

CORS is fully open (`AllowOrigin::any()`) to support browser WASM clients connecting cross-origin. `tonic-web` + `GrpcWebLayer` allow gRPC-web connections from browsers.

### DevTools watch client (`src/devtools_watch.rs`, WASM only)

The web GUI subscribes to a devtools server's `Watch` stream via `tonic-web-wasm-client` and dispatches received changes into its store. The server URL is the page origin (same-origin behind a reverse proxy); a `localhost`/`127.*` origin (plain `dx serve`) falls back to `http://localhost:49051`. Reconnects with exponential backoff; replayed history is deduplicated via a per-app high-water counter. On WASM the GUI does not monitor itself (`with_devtools(false)`).

### Views and Facade pattern (`src/views/`)

Each view has a paired **facade** struct that mediates between the Dioxus component and the Redux store. Components receive the store via Dioxus context (`use_context::<Store>`) and construct a facade with `use_signal`.

- **`HomeView`** (layout): sidebar drawer listing connected apps. Dispatches `Action::SelectedAppChange`.
- **`AppStateView`** (route `/data/app/:app_id`): hosts `StatesList` and `StateExplorer` side by side. Renders tabs for Tree / JSON / RON viewer modes.
- **`StatesList`**: lazy-rendered list of action history entries (newest first). Each row is an `ActionListItem`; clicking dispatches `Action::SelectedStateChange`.
- **`StateExplorer`**: displays the selected action + state in the chosen format. Tree mode uses recursive `StateValueRon` / `StateItemValueRon` components that render `ron::Value` as a collapsible DaisyUI menu.

### Routing

```
/data/app/:app_id   → AppStateView (inside HomeView layout)
/data/              → redirect to AppStateView { app_id: "" }
/:..segments        → NotFoundView → redirect to /data
```

### Tailwind / DaisyUI

Compiled CSS lives at `assets/tailwind.css` (committed). Source is `input.css`; DaisyUI is included as a plugin. Run `npm run css:watch` whenever adding new utility classes.

### Configuration persistence

Desktop config path: resolved via `directories::ProjectDirs::from("net", "zoechbauer", "bwu_redux_devtools")` → `{config_dir}/settings.toml`. Config values can also be set via environment variables prefixed `BWU_REDUX_` (handled by the `config` crate).
