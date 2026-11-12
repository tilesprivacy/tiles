default: check

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets -- -D warnings

check:
    just fmt
    just lint
    cargo test
    # just py_test

serve:
    server/.venv/bin/python3 -m server.main
    # uv run --project server python -m server.main

bundle *ARGS:
    ./scripts/bundler.sh {{ARGS}}

install *ARGS:
    ./scripts/install.sh {{ARGS}}

bundle_pkg:
    ./pkg/build.sh
    ./pkg/bundle_network_installer.sh

bundle_model_pkg:
    ./pkg/build_model.sh

bundle_pkg_full:
    ./pkg/build.sh
    ./pkg/build_full.sh

# like `just bump 0.5.0-rc.2`. a final version also dates its CHANGELOG section
bump VERSION:
    sed -i.bak 's/^version = ".*"/version = "{{VERSION}}"/' tiles/Cargo.toml && rm tiles/Cargo.toml.bak
    cargo check -q -p tiles
    case "{{VERSION}}" in *-*) ;; *) sed -i.bak "s/^## \[{{VERSION}}\] - Unreleased/## [{{VERSION}}] - $(date +%Y-%m-%d)/" CHANGELOG.md && rm CHANGELOG.md.bak ;; esac

build_w_pi:
    ./scripts/build_with_pi_dev.sh

py_test:
    uv run --project server pytest server/tests
    
# runtiles: RUST_LOG=tiles=info,iroh=off cargo run
