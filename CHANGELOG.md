# Changelog

All notable changes to this project are documented in this file.  
The format is based on https://keepachangelog.com/en/1.1.0/

## [Unreleased]

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
