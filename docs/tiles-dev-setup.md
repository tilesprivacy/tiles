# Tiles Dev Setup

Everything you need to build, run, and test the Tiles stack — daemon, chat UI, and menu bar app — wired together in one dev environment. Written 2026-09-10, after the plugins/session-switching work landed on `canary`.

## The moving parts

```
tiles-ui (Svelte SPA)          apps/menubar (Tauri)
   vite dev :5173  ──────┐        │ supervised by the daemon
   or bundled into app   │        │ (lifeline stdin pipe)
                         ▼        ▼
              tiles daemon  http://127.0.0.1:1729
                         │
              Pi agent (ONE process, ONE live conversation,
              the daemon switches it between sessions)
                         │
              python server → llama-server (inference)
```

Things that are easy to get wrong, up front:

- **The daemon owns everything.** The menu bar app does not own the daemon: a hand-launched app starts `tiles daemon` if none answers on 1729, and the daemon then spawns its own supervised copy of the app and takes over (`apps/menubar/src-tauri/src/boot.rs`).
- **There is exactly one Pi process** for the whole daemon, holding one conversation. Prompts carry a `session_id`; when it differs from the session Pi is on, the daemon resets Pi and replays that session's stored history (`tiles/src/daemon/agent.rs`).
- **Debug builds and release builds live in different worlds.** `cargo run` (debug) uses `<repo>/.tiles_dev/tiles/` for everything and resolves it from the *current working directory*, so always run from the repo root. Release/installed builds use `/usr/local/share/tiles` (lib) and `~/.local/share/tiles/data` (data).

## Repos and branches

| Repo | Branch to work on | What lives there |
|---|---|---|
| `tilesprivacy/tiles` | `canary` | daemon, CLI/REPL, menu bar app, packaging, bundled plugins (`plugins/exa`), vendored MCP adapter (`vendor/`) |
| `tilesprivacy/tiles-ui` | `main` | the chat UI (SvelteKit SPA) |

Clone them side by side; the menubar bundling script can point at your local UI checkout:

```sh
git clone https://github.com/tilesprivacy/tiles.git
git clone https://github.com/tilesprivacy/tiles-ui.git
cd tiles && git checkout canary
```

Two traps:

- A **tag named `canary`** exists (from the prerelease), so bare `canary` is ambiguous to git. Use `origin/canary` in scripts, or ignore the warning.
- `tiles` releases cut from `main`; the recent work (plugins, `@name`, session switching, boot logging) is **only on `canary`**.

## Prerequisites

- Rust + Cargo (toolchain pinned in `rust-toolchain.toml`), `just`
- Python 3.13 + `uv`
- Node 20+, `npm` (tiles-ui uses a package-lock) and `pnpm` (the menubar workspace)
- macOS for the menu bar app
- For packaging only: `venvstacks`, a Developer ID Application + Installer certificate, and notarytool credentials

## One-time setup

### tiles (daemon/CLI)

From the `tiles` repo root:

```sh
cargo build
(cd server && uv sync)              # python inference server deps
./scripts/fetch_llama_server.sh     # llama-server binary into server/bin
./scripts/setup_dev_layout.sh       # symlinks dev assets into .tiles_dev/tiles/
just build_w_pi                     # downloads the Pi binary into .tiles_dev/tiles/pi
```

The default model (`unsloth/gemma-4-12b-it-GGUF` Q4_K_M) downloads on the first run.

Create the local account once (chats can't be saved without it, and the DBs are SQLCipher-encrypted with a key stored against it in the login keychain):

```sh
cargo run -- account create <nickname>
```

**Keychain matters:** the daemon reads the device key from the login keychain at startup and dies without it. If a fresh daemon silently won't start, that's the first suspect — check the log (below).

### tiles-ui

```sh
cd tiles-ui
npm ci
```

## Running everything together (dev)

Three terminals, all connecting through port 1729:

```sh
# 1 — inference server
cd tiles && just serve

# 2 — daemon (from the repo ROOT: debug builds resolve .tiles_dev from cwd)
cd tiles && cargo run -- daemon --no-ui

# 3 — chat UI
cd tiles-ui && npm run dev        # http://localhost:5173
```

How the UI finds the daemon: in dev, `API_ORIGIN` is empty (same origin) and vite proxies `/v1` → `http://127.0.0.1:1729` (override with `VITE_PUBLIC_SERVER_ORIGIN`). The daemon's CORS allowlist already includes `localhost:5173`.

To also run the menu bar app against the same daemon:

```sh
cd tiles && pnpm install && pnpm --filter tiles-menubar tauri dev
```

The app only watches and reports daemon health; it won't fight your terminal-run daemon. (`TILES_CLI_BIN` overrides which binary a hand-launched app would boot.)

## Running everything together (packaged, what users get)

```sh
cd tiles && just bundle_pkg       # → pkg/tiles.pkg (signed + notarized)
sudo installer -pkg pkg/tiles.pkg -target /
open /Applications/Tiles.app
```

- The build clones tiles-ui at **whatever its default branch points to** (`apps/menubar/ui.pin` says `latest`) and prints `chat ui: <sha>` — that log line is the only record of which UI a package carries. Check it.
- To bundle your local UI checkout instead: `TILES_UI_DIR=/path/to/tiles-ui just bundle_pkg`.
- Launch order: app → boots `/usr/local/bin/tiles daemon` → daemon respawns the app supervised → chat window appears.

## What to test (the recent work)

1. **Plugins from the UI.** Type `@` in the chat box: the picker lists `exa` (plugin) and `web-research` (skill) above file results. `@exa` alone answers instantly with a description (no model turn); `@exa top news today` runs a real turn with the web-search tool call rendering live. An unknown `@name` answers with what's available.
2. **Session isolation.** Tell chat A "my favorite fruit is pineapple", make chat B and mention mango, go back to A and ask "what's my favorite fruit?" — must answer pineapple. (Watch the daemon log for `skipping event while waiting for new_session` — that's the switch machinery working past the MCP adapter's chatter.)
3. **Mid-generation tab switch.** Start a long generation (e.g. "count from 1 to 200, one per line"), switch to another chat mid-stream, switch back: the text must still be there *and still growing*, with the reasoning block and model badge intact. After completion, reasoning + tool calls must survive any amount of switching.
4. **Sharing.** Share a session (needs `tiles login <handle>` / ATproto): the published snapshot must carry the model name, thinking, and tool calls — not just final text.

There's a Playwright repro for #3 (drives the dev UI at :5173) worth resurrecting into `tests/e2e` — pattern: send prompt, wait for the Reasoning block, client-side navigate away and back, assert text keeps growing.

## Debugging cheatsheet

**Ports & endpoints** (daemon = `http://127.0.0.1:1729`):

```sh
curl :1729/                                   # daemon version = alive
curl :1729/v1/tilekit/agent/state             # Pi session id, model, thinking level
curl :1729/v1/tilekit/agent/commands          # everything @name can reach
curl -X POST :1729/v1/tilekit/session/new     # new session (422 = Pi call failed)
curl :1729/v1/tilekit/session/list
curl -N -X POST :1729/v1/tilekit/agent/prompt \
  -H 'content-type: application/json' \
  -d '{"message":"hi","session_id":"<sid>"}'  # SSE stream of Pi events
```

**Logs** (installed builds; dev logs go to the daemon terminal):

- `~/.local/share/tiles/data/logs/boot.log` — the daemon's stdout/stderr when the app booted it. **Pi's dying words land here** (Pi inherits the daemon's stderr). First stop for "daemon won't start" and "Failed to send to Pi's stdin".
- `~/.local/share/tiles/data/logs/daemon.{out,err}.log` — CLI-respawned daemons.
- `~/.local/share/tiles/data/logs/llama-server.err.log` — inference.

**Symptom table** (all earned the hard way):

| Symptom | Likely cause |
|---|---|
| "Failed to send to Pi's stdin" on every prompt | Pi died at startup — read boot.log. Historically: a vendored package missing files (`.gitignore` ate `dist/`), or an unsigned native module under hardened runtime |
| `/session/new` returns 422 | A Pi RPC failed — usually a reader colliding with extension events on Pi's stdout; every read must go through the event-skipping `request()` loop in `core/agent/pi.rs` |
| Daemon answers `/` but agent/session endpoints hang | A task wedged holding the agent lock (a reader waiting on a line Pi will never send). `sample <daemon-pid>` to confirm; restart; find the desynced reader |
| Wrong-context answers after switching chats | The session-switch replay isn't running — check for `Could not switch Pi to session` in the log |
| UI shows stale/empty thread after switching | The load merges daemon rows with the local IndexedDB copy (`overlayLocalMessages` in tiles-ui) — daemon rows carry only role/text/model; reasoning + tool calls + in-flight turns come from the local row |

**Data locations:** chats DB `~/.local/share/tiles/data/chats_v2.db` (SQLCipher-encrypted, key in the login keychain — you can't sqlite3 into it; delete the file to wipe, the daemon recreates it). UI-side cache is IndexedDB `Tiles` inside the webview.

## Architecture notes worth knowing before touching code

- **Rows vs snapshots.** The daemon's chat rows store only role/final-text/model. The rich record (thinking, tool calls) lives in the per-session *snapshot*, built from Pi's reported turns — that's what sharing publishes. Anything rebuilt from rows alone loses reasoning by construction.
- **`@name` is resolved by the daemon** (`core/plugin.rs::resolve_invocation`), shared with the REPL. The UI just sends `@name …` as plain prompt text; the picker is discovery only.
- **Every Pi RPC must tolerate extension events.** The bundled MCP adapter announces itself on Pi's stdout at startup *and* at session reset. Never read one line and expect your response.
- **`vendor/node_modules` is committed on purpose** and shielded from `.gitignore` by explicit `!vendor/node_modules/` re-includes. If you touch the ignore rules, verify with `git status --ignored vendor`. Refresh the tree with `vendor/refresh.sh`.
- The pkg's `cp -r server` sweeps whatever is in your working tree — a dirty tree ships junk (a 0.4.19 release once shipped 22 MB of coverage files this way).
