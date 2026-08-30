# Changelog

All notable changes to this project are documented in this file.  
The format is based on https://keepachangelog.com/en/1.1.0/

## [Unreleased]

## [0.4.19] - 2026-08-30

### Added

- `--mtp` flag for `tiles run` to enable/disable MTP speculative decoding (persists to `config.toml` like the other llama flags).

- Model-load warnings from the inference server (e.g. MTP requested but no MTP head GGUF found) are now surfaced in the CLI as yellow `WARNING:` lines before the input prompt.

### Changed

- MTP speculative decoding is now opt-in: it no longer auto-enables when an MTP head GGUF is detected next to the model. Enable it with `mtp = true` under `[llama]` in `config.toml` or per-run with `tiles run --mtp`.

### Fixed

- Chances of installing multiple version of Tiles during `tiles update` #[194](https://github.com/tilesprivacy/tiles/pull/194)


## [0.4.18] - 2026-08-23

### Added

- Vulkan backend for llama.cpp. Now Tiles can leverage GPUs other than nvidia [#189](https://github.com/tilesprivacy/tiles/pull/189)


### Fixed

- MTP was disabled due to hardcode symlink [#187](https://github.com/tilesprivacy/tiles/pull/187)

- (linux) - Tool calls were returning 422s due to wrong types caused due to a version difference in openresponses-types [#188](https://github.com/tilesprivacy/tiles/pull/188)

- OAuth callback was not working sometimes during ATproto login due to child process for opening browser doesn't resolve immediately [#190](https://github.com/tilesprivacy/tiles/pull/190)

@bruhfr29 FTW

## [0.4.17] - 2026-08-18

### Added

- Quantization selection in modelfiles via ollama-style tag: `FROM unsloth/gemma-4-12b-it-GGUF:Q8_0` (defaults to `Q4_K_M` when no tag is given).
- MTP speculative decoding now auto-enables when the model ships an MTP head; disable with `[llama] mtp = false` in config.toml.
- Linux release CI: any pushed tag (stable/rc/nightly) builds the x86_64 tarball and attaches it (plus sha256 checksums) to the tag's GitHub Release.
- Offline (full) installer build is now fully scripted: `pkg/build_model.sh` bundles the default Gemma GGUF + MTP head.

### Changed

- Default model switched to `unsloth/gemma-4-12b-it-GGUF` (Q4_K_M).
- Upgraded Pi to v0.84.2 (upstream `badlogic/pi-mono`), dropping the stale tiles-pi fork pin.

### Fixed

- macOS notarization: Pi's bundled `.node` native modules are now signed (unsigned modules shipped with Pi v0.84.x broke notarization).
- Treat a missing GGUF inside a cached model snapshot as a partial download and re-download instead of failing.

## [0.4.16] - 2026-07-11

### Added

- Switching to llama server from custon inference server

### Added

## [0.4.15] - 2026-07-07

### Added

- Added `tiles uninstall` command to uninstall tiles from the system [#173](https://github.com/tilesprivacy/tiles/pull/173)
  - By default it will keep the user data folder and config.toml. If pass `--all` will do a complete cleanup.

- Added ATproto lexicon support for the shared sessions. The lexicon for that can be found [here](https://lexicon.garden/lexicon/did:plc:mqmcsjuerbjhu65mpmvkcuw2/run.tiles.chat.sessionSnapshot) [#175](https://github.com/tilesprivacy/tiles/pull/175)

### Fixed
- Optimizations in ToolCalling by adding timeouts and removing non-recursive syscalls + preventing 422 errors in /v1/responses api [#174](https://github.com/tilesprivacy/tiles/pull/174).
- Set Pi's compaction setting to false by default to prevent it interfering with prompt/response cycle [#174](https://github.com/tilesprivacy/tiles/pull/174).

## [0.4.14] - 2026-06-28

### Added

- Implemented Tiles plugin system [#171](https://github.com/tilesprivacy/tiles/pull/171).
  - `tiles plugin install <url / filesystem-path>`, `tiles plugin uninstall <plugin-name>`, `tiles plugin list` (for installed plugins).
    - Plugins should be a `.zip` or `.tar.gz` file either hosted or available in local filesystem.
    ```markdown

      ## Plugin folder structure

      plugin_name
        - extensions
          - extension_1
            - ...
          - extension_2
            - ...
        - skills
          - skill_1
            - SKILLS.md
      ```
    - Added skills support via plugins
      - In repl use `/skills` for list of skills and `$<skill-name>` to use the skill directly. Tiles can use available skills as needed too.

### Changed

- Upgraded project dependencies [#168](https://github.com/tilesprivacy/tiles/pull/168), [#170](https://github.com/tilesprivacy/tiles/pull/170).

## [0.4.13] - 2026-06-19

### Added
- Added `tiles run` flags for llama.cpp tuning: `--context-length`, `--gpu-layers`, `--offload-kqv`, and `--batch-size` [#160](https://github.com/tilesprivacy/tiles/pull/160).
- Added a daemon `/config` endpoint so the Python inference backend can read Rust-owned Tiles config [#160](https://github.com/tilesprivacy/tiles/pull/160).

### Changed
- Persist llama.cpp settings under `[llama]` in `config.toml` instead of using `TILES_LLAMA_CPP_*` environment variables [#160](https://github.com/tilesprivacy/tiles/pull/160).
- Reload the Linux llama.cpp runner when llama configuration changes, even if the selected model path stays the same [#160](https://github.com/tilesprivacy/tiles/pull/160).
- Reworked link command UX around `tiles link create`, `tiles link add`, `tiles link list-peers`, and `tiles link revoke`, with support for both offline link codes and UCAN tokens [#163](https://github.com/tilesprivacy/tiles/pull/163).
- Renamed inference system controls to `tiles system server` with `start`, `stop`, and `daemon` subcommands [#163](https://github.com/tilesprivacy/tiles/pull/163).
- Switched Harmony handling to `tiles-harmony` across active server manifests [#162](https://github.com/tilesprivacy/tiles/pull/162).

### Fixed
- Improved GPT-OSS tool call handling by normalizing malformed tool names, passing tool metadata into Harmony conversation replay, and detecting tool calls emitted through the analysis channel [#162](https://github.com/tilesprivacy/tiles/pull/162).
- Fixed tool-call streaming state handling so final answers and function-call arguments are emitted more reliably [#162](https://github.com/tilesprivacy/tiles/pull/162).
- Improved dev Modelfile handling so `cargo run -- run modelfiles/gpt-oss-gguf` no longer depends on a pre-existing copied default Modelfile [#160](https://github.com/tilesprivacy/tiles/pull/160).

## [0.4.12] - 2026-06-15

### Added
- Added full support for Linux. Model Inference(llama.cpp), keychain management, Installer etc [#138](https://github.com/tilesprivacy/tiles/pull/138).
- Using UCAN based capability tokens for authorization in p2p syncing, thus replacing the need for synchronous peer linking [#154](https://github.com/tilesprivacy/tiles/pull/154)
    - Added two new sub commands under `tiles link`. `tiles link create-token <DID>`, which creates a UCAN token for the DID. `tiles link add-token <token>`, adds a given UCAN token to local DB for further use in in syncing.

### Changed
- Changed `tiles server` command to `tiles inference` with a newly added sub-command `run-background` which takes a boolean value. If true, closing the Tiles repl won't close the inference [#156](https://github.com/tilesprivacy/tiles/pull/156), [#157](https://github.com/tilesprivacy/tiles/pull/157)
- Added extra metadata regarding tool and the arguments used in a tool-call to the session records, thus can be seen in Tiles sessions hosted in ATproto PDS [#155](https://github.com/tilesprivacy/tiles/pull/155)

## [0.4.11] - 2026-06-07

### Added
- New repl command `/reasoning` to switch reasoning effort on the fly [#150](https://github.com/tilesprivacy/tiles/pull/153), [#153](https://github.com/tilesprivacy/tiles/pull/153).
  - eg: `/reasoning low` - sets effort to low. Other options are medium, high

### Fixed
- Broken repl behaviour on SIGINT (ctr-c) [#152](https://github.com/tilesprivacy/tiles/pull/152).

### Changed
- Using the tool_call name model gives, instead of figuring out from the tool_call arguments [#151](https://github.com/tilesprivacy/tiles/pull/151).

## [0.4.10] - 2026-05-31

### Added
- Implemented Tool calls [#144](https://github.com/tilesprivacy/tiles/pull/144)
  - Support READ, WRITE, BASH, EDIT by default

### Fixed

- Handling the DNS resolver err and showing freindly user msg [#143](https://github.com/tilesprivacy/tiles/pull/143)

### Changed

- Major refactor related to Inference and REPL [#140](https://github.com/tilesprivacy/tiles/pull/140)

## [0.4.9] - 2026-05-22

### Added

- Added encrypted session sharing over ATproto [#141](https://github.com/tilesprivacy/tiles/pull/141)

    - Use the same `/share` command, but Tiles will ask for public or private sharing.

### Changed

- Routine refactoring and error handling [#139](https://github.com/tilesprivacy/tiles/pull/139)


## [0.4.8] - 2026-05-01

## Added
- Integrate Pi for Agent Harnerss via embedding [#126](https://github.com/tilesprivacy/tiles/pull/126)

- Added Sessions feature [#126](https://github.com/tilesprivacy/tiles/pull/126)
- Added Atproto login [#129](https://github.com/tilesprivacy/tiles/pull/129)
    - `tiles at login <handle>`
    - `tiles at logout`

- Added repl commands for session management [#132](https://github.com/tilesprivacy/tiles/pull/132)

    - `/sessions` - List sessions
    - `/share` - share current session by writing to PDS
    - `/share <sessionId>` - share particular session by writing to PDS
    - `/resume <sessionId>` - Load and continue a particular session

- Added `/status` repl command for showing current session status [#134](https://github.com/tilesprivacy/tiles/pull/134)


## Changed

* Multiple UI/UX improvements and refactoring in [#127](https://github.com/tilesprivacy/tiles/pull/127), [#131](https://github.com/tilesprivacy/tiles/pull/131), [#133](https://github.com/tilesprivacy/tiles/pull/133), [#135](https://github.com/tilesprivacy/tiles/pull/127)
 

## [0.4.6] - 2026-03-30

### Added
- Added P2P chat sync in [#109](https://github.com/tilesprivacy/tiles/pull/109)
  - Commands for chat syncing
    - `tiles sync` - Starts listening for a sync request from the linked peers.
    - `tiles sync <DID>` - Initiates the syncing with the peer using the peer's linked DID (which one can get from `tiles link list-peers`).
- Added at rest encryption for local databases in [#110](https://github.com/tilesprivacy/tiles/pull/110)

### Fixed
- Fixed the loading issue of qwen 3.5 series in [#111](https://github.com/tilesprivacy/tiles/pull/111)

## [0.4.5] - 2026-03-23

### Added
- Added P2P device linking v1 in [#106](https://github.com/tilesprivacy/tiles/pull/106).
  - Works both online and in offline networks
  - Utility Commands for device linking
    - `tiles link enable` - creates the ticket and listens for an link requests
    - `tiles link enable <ticket>`- Device that need to join will run this command with the ticket from the sender. **NOTE**: The ticket sharing is out-of-band.
    - `tiles link list-peers` - Shows the info (DID, nickname etc) of the linked devices.
    - `tiles link disable <DID>` - Unlinks a linked device 

### Fixed
- Fixed the permission issues while trying to update Tiles using `tiles update` in [$104](https://github.com/tilesprivacy/tiles/pull/104). This was due to new binary location is in `/usr/` instead of `~/.local/`. Running the internal script with `sudo` fixed it.

## [0.4.4] - 2026-03-16

### Added
- Added a core daemon process which will be useful for handling background processes in https://github.com/tilesprivacy/tiles/pull/102

    - Use `tiles daemon stop` and `tiles daemon start` for starting and stopping the daemon explicitly. NOTE: daemon will auto-start when you run `tiles`.

- Added support for fully offline/portable installer in https://github.com/tilesprivacy/tiles/pull/97


## [0.4.3] - 2026-03-08

### Added
- Chats are now persisted using sqlite underneath by @madclaws in https://github.com/tilesprivacy/tiles/pull/94

- Release artifacts will have .pkg bundles too for easy installs by @madclaws in https://github.com/tilesprivacy/tiles/pull/96

### Changed
-  Inference enhancements by @madclaws in https://github.com/tilesprivacy/tiles/pull/95
   - Support for non-harmony response models
   - Supports turn conversation with the model

### Fixed
- fixed venvstack generating multiple app-server tar on build by @madclaws in https://github.com/tilesprivacy/tiles/pull/93


## [0.4.2] - 2026-03-01
### Added
- Added FTUE changes for account setup in https://github.com/tilesprivacy/tiles/pull/88
- Added OTA updater in https://github.com/tilesprivacy/tiles/pull/89
  - Supports auto update checking and installing
  - Use `tiles update` for updating Tiles CLI manually

### Changed
- Integrated Harmony renderer for gpt-oss model in https://github.com/tilesprivacy/tiles/pull/92

### Fixed
- fix: Added path unavailability warning during installation in https://github.com/tilesprivacy/tiles/pull/90
- coverage patch-1 in @https://github.com/tilesprivacy/tiles/pull/91

## [0.4.1] - 2026-02-22
### Added
- Identity system for Tiles:
  - `tiles account` to show account details
  - `tiles account create <nickname>` to create root identity and optional nickname
  - `tiles account set-nickname` to set a nickname for root identity
- Updated CLI to include default `tiles` command

## [0.4.0] - 2026-02-04
### Added
- Portable Python runtime in the installer (no system Python required)
- Bundled default Modelfiles and direct reading of system prompt from Modelfile
- Support for `gpt-oss-20b` in interactive chat
- Basic support for the Open Responses API (`/v1/responses`) and REST endpoints
- Token metrics for model responses in the REPL
- `-m` flag for `tiles run` to execute Tiles in memory mode (experimental)
- Tilekit 0.2.0: `optimize` subcommand for automatic system-prompt optimization via DSRs

## [0.3.1] - 2026-01-09
### Added
- `--relay-count` / `-r` option for `tiles run` (helps if model gets stuck)
- CLI shows progress status while downloading models
- Slash commands and placeholder hint in the REPL
- Ability to set custom memory location via `tiles memory set-path <PATH>`

### Changed
- Minor internal refactoring

## [0.3.0] - 2026-01-06
### Fixed
- Tiles binary startup issue when run from outside a project directory
- Model not unloading after exiting the REPL
- Updated Python version to 3.13 for development
- Enabled basic Linux compatibility

### Changed
- Basic refactoring to support multiple inference runtimes

## [0.2.0] - 2025-12-20
### Added
- Server commands
- Streaming support with “thinking tokens” in the CLI
- Auto-downloading of model specified in Modelfile
