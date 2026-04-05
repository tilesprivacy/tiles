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

### Llama.cpp Web UI (optional)

To use the upstream [llama.cpp](https://github.com/ggml-org/llama.cpp) SvelteKit chat UI against the Tiles Python server:

1. Keep **their** repository read-only from Tiles’ perspective: clone or add a **git submodule** (e.g. at `third_party/llama.cpp`), then `git pull` or bump the submodule revision when you want their updates. Do not push Tiles-only commits to `ggml-org/llama.cpp`.
2. Start Tiles (`just serve`, default `http://127.0.0.1:6969`).
3. From the Tiles repo root, run `scripts/phase2_llamacpp_webui.sh`. It resolves `llama.cpp` in this order: `LLAMA_CPP_ROOT`, then `third_party/llama.cpp`, then a sibling `../llama.cpp` clone. It patches `tools/server/webui/vite.config.ts` to proxy API routes to the Tiles backend when needed.

   Optional submodule (pinned revision in your Tiles fork/branch):

   ```sh
   git submodule add https://github.com/ggml-org/llama.cpp third_party/llama.cpp
   git submodule update --init --recursive
   ```

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

## Additional Resources

- [Tiles Book](https://tiles.run/book)
- [Download Page](https://tiles.run/download)
- [Community & Support](https://go.tiles.run/discord)
