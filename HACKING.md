# HACKING.md

This guide will help you set up a reproducible development environment for Tiles. Tiles supports two environments: `prod` (production) and `dev` (development). These instructions assume you are setting up for local development.

## Prerequisites

- [Rust & Cargo](https://www.rust-lang.org/tools/install)
- [`just`](https://github.com/casey/just) (for task management)
- [Python 3.13](https://www.python.org/downloads/)
- [`uv`](https://docs.astral.sh/uv/) (for fast Python dependency management)
- [Git](https://git-scm.com/)

## Setup Steps

1. **Clone the repository:**

   ```sh
   git clone https://github.com/tilesprivacy/tiles.git
   cd tiles
   ```

2. **Install Rust dependencies:**

   If you're new to Rust, see [Rust Install Guide](https://www.rust-lang.org/tools/install).

   ```sh
   cargo build
   ```

3. **Install project task runner:**

   [`just`](https://github.com/casey/just) provides easy command shortcuts.

   ```sh
   cargo install just      # or use your OS package manager
   ```

4. **Set up the Python server environment:**

   - Make sure [`uv`](https://docs.astral.sh/uv/) is installed:

     ```sh
     pip install uv
     ```

   - Sync Python dependencies:

     ```sh
     cd server
     uv sync
     cd ..
     ```

## Running Tiles (Development)

Open two terminal windows:

1. **Terminal 1: Start the server**

   From the project root:

   ```sh
   just serve
   ```

2. **Terminal 2: Run the Rust CLI**

   From the root directory:

   ```sh
   cargo run

   ```

> **Tip:** Refer to the `justfile` for additional common commands and automation. For troubleshooting, see [CONTRIBUTING.md](CONTRIBUTING.md) and open an issue if you need help.

### Building Tiles installer (Development)

Install [venvstacks](https://github.com/lmstudio-ai/venvstacks?tab=readme-ov-file#installing) for portable py runtime

From the project root do,

```sh
just bundle # Creates the compressed zip in dist/
```

Set the `ENV` in install.sh to `dev`

```sh
just install
```

Now `tiles` should be available in PATH


### Development with PI

[Pi](https://github.com/badlogic/pi-mono) is a minimal coding agent for agentic harness. Instead of providing harness by ourselves we will be leveraging Pi.


Current approach on how we integrate Pi is, we pack the pi bun binary with the tiles installer and we switch to Pi repl from tiles cli, if harness is required. There are two ways we can do communicate with Pi, either via rpc mode or directly use the pi binary and get into the whole Pi ecosystem.

For better maintainability and to be update with Pi, using rpc mode is the way. But as we are in experimental mode with Pi, for now we wont use rpc instead completely use Pi's repl and UI. So at this stage we use our own fork for tighter integration with Tiles system. But this can change later. So Pi will be available under a flag `tiles -p` or `tiles run -p <MODELFILE_PATH>`


#### Setting up PI

`git clone https://github.com/tilesprivacy/tiles-pi/tree/feat/integrate-w-tiles`

`npm install` - for installing the deps

```
export TILES_PI_BUILD_ENV=debug # (other values: release)
export TILES_PI_DEV_CONFIG_PATH=<TILES_REPO_PATH>/.tiles_dev/tiles
```

Set these env vars. `TILES_PI_BUILD_ENV` is used to find the correct config.toml file for Pi to read. tiles-pi rely on config.toml for user data directory, current model etc. At this point config.toml act as a shared memory for tiles-pi and tiles. For development we use `debug` value. If debug mode then it uses `TILES_PI_DEV_CONFIG_PATH`. So internally all the app-files, user-data etc are in a .tiles_dev folder at the root of project. Pi also creates it agent directory here under `.tiles_dev/tiles/data/pi/agent`.

If mode is anything other than debug, then its release mode and the config.toml path is fixed, so need to worry about. Important thing to note is pi/agent directory will be in the tiles user data directory.

To work with Pi we need to run Pi on a terminal and tiles inference py server on another, and tiles daemon shld also be running background.

- Running Pi
   - From root of `tiles-pi` run `npm run build && ./pi-test.sh`
- Running py server
   - From root of `tiles` run `just serve`
- Running tiles daemon
   - First check if daemon is already running by `curl -X GET http://127.0.0.1:1729/`, if its returning tiles version, then daemon is running and its fine.

   - If above curl failed, then do `cargo  run -- -x`. This will run tiles in non-repl mode, simultaneously running a deamon in background.

Now these are running, u can jump into pi repl and do stuff with the model

Later if we need to test the e2e integration in development, we need to build the tiles-pi binary and extract the artificats into `.tiles_dev/tiles/pi`.
For that we can run `just build_w_pi`.


## Additional Resources

- [Tiles Book](https://tiles.run/book)
- [Download Page](https://tiles.run/download)
- [Community & Support](https://go.tiles.run/discord)
