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

5. **Provision the inference engine:**

   Tiles runs inference through a `llama-server` binary. Fetch it for your platform:

   ```sh
   ./scripts/fetch_llama_server.sh
   ```

   - macOS: downloads the official prebuilt Metal release into `server/bin/`.
   - Linux: builds `llama-server` from source, with CUDA when `nvcc` is available (CPU otherwise).

   Then symlink the dev assets into `.tiles_dev/tiles/`:

   ```sh
   ./scripts/setup_dev_layout.sh
   ```

   The default model (`unsloth/gemma-4-12b-it-GGUF`, Q4_K_M) is downloaded automatically on the first `cargo run`. A different quantization can be selected in the modelfile with `FROM unsloth/gemma-4-12b-it-GGUF:Q8_0` (ollama-style tag). MTP speculative decoding auto-enables when the model ships an MTP head (`mtp-*.gguf` next to the main model); disable with `[llama] mtp = false` in config.toml.

### Embedding Pi

[Pi](https://github.com/badlogic/pi-mono) is a minimal coding agent for agentic harness. We embed Pi in Tiles so that it can sit in between the CLI and inference layer to provide more powerful features for the regular knowledge work, agent harness and whatever that comes in future is just an extension away thus making Tiles flexible and can ride the wave of standards.

Current approach on how we integrate Pi is, we pack the pi bun binary with the tiles installer and use Pi in rpc mode from Tiles. So Pi interacts with the Tiles model inference and communicates with the Tiles Pi via stdin, stdout json.


#### Setting up PI

For development,  Tiles expect a `pi` folder under `.tiles_dev/tiles/` (This folder is created first time when we run tiles from the root directory with `cargo run`). We can run `just build_w_pi` which will handle downloading and extracting the relevant Pi binary.



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

## Running the menu bar app (Development)

The menu bar app lives in `apps/menubar`. It needs [`pnpm`](https://pnpm.io/installation).

Open two terminal windows:

1. **Terminal 1: Start the daemon**

   From the project root:

   ```sh
   cargo run -- daemon --no-ui
   ```

2. **Terminal 2: Run the app**

   From the root directory:

   ```sh
   pnpm --filter tiles-menubar tauri dev
   ```

## Building Tiles installer (Development)

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


## Additional Resources

- [Tiles Book](https://tiles.run/book)
- [Download Page](https://tiles.run/download)
- [Community & Support](https://go.tiles.run/discord)
