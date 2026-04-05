default: check

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets -- -D warnings

check:
    just fmt
    just lint
    cargo test

serve:
    server/.venv/bin/python3 -m server.main
    # uv run --project server python -m server.main

# Python server (OpenAI compat API tests; requires uv sync in server/)
test-server:
    uv run --project server pytest server/tests/ -v

# llama.cpp SvelteKit UI (clone under ../llama.cpp by default); proxies to Tiles :6969
webui-llamacpp:
    bash scripts/phase2_llamacpp_webui.sh

bundle:
    ./scripts/bundler.sh

install:
    ./scripts/install.sh

bundle_pkg:
    ./pkg/build.sh
    ./pkg/bundle_network_installer.sh

bundle_model_pkg:
    ./pkg/build_model.sh

bundle_pkg_full:
    ./pkg/build.sh
    ./pkg/build_full.sh

# runtiles: RUST_LOG=tiles=info,iroh=off cargo run
