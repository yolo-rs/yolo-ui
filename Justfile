# =============================================================================
# Yolo UI (`yolo-rs/yolo-ui`) — End-to-End Task Runner (`Justfile`)
# =============================================================================
# Unified orchestration for Cargo + sccache + Bun v1.4.4+ + n2b + GitHub CLI (gh)

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# List all available recipes
default:
    @just --list

# Verify toolchain (bun, cargo, rustc, sccache, just, gh)
doctor:
    bun --version
    cargo --version
    rustc --version
    sccache --version
    just --version
    gh --version

# Format Rust crates and Bun TypeScript scripts/tests
fmt:
    cargo fmt --all
    bun fmt scripts tests

# Check formatting and lints across Rust and Bun TypeScript
lint:
    bun fmt --check scripts tests
    bun lint scripts tests
    bun check
    cargo clippy --workspace -- -D warnings

# Run fast typecheck and metadata validation across Bun & Cargo
check:
    bun check
    cargo metadata --no-deps --format-version 1 > $null
    cargo check

# Run all Bun and Rust unit tests
test: test-bun test-rust

# Run Bun v1.4.4+ monorepo test suite
test-bun:
    bun test

# Run Rust unit tests across core and multi-engine crates
test-rust:
    cargo test --workspace

# Build core crates and showcase app
build:
    bun scripts/build.ts

# Build release artifacts
build-release:
    bun scripts/build.ts --release

# Run the unified multi-engine showcase binary (`apps/yolo-showcase`)
showcase:
    cargo run -p yolo-showcase

# Unified UI CLI (search, system, theme, migrate, contrast, devtools, awesome, catalog, mcp, doctor)
ui *ARGS:
    bun scripts/ui.ts {{ARGS}}

# Full-Bun + Rust React 19 & Expo CLI (create, purge-node, patch-expo, build, status)
react *ARGS:
    bun scripts/react-bun.ts {{ARGS}}

# Inspect status of the configured source submodule fleet
fleet-status:
    bun scripts/fleet.ts status

# Shallow-sync the configured source submodule fleet (`--depth 1`)
fleet-sync:
    bun scripts/fleet.ts sync

# Generate `fleet.json` inventory for the configured source submodule fleet
fleet-inventory:
    bun scripts/fleet.ts inventory

# Audit and optimize Cargo workspace dependencies, profiles, and build steps
optimize:
    bun scripts/optimize.ts

# Scan and migrate Node/npm/pnpm/yarn patterns to native Bun v1.4.4+ (`n2b`)
n2b-migrate:
    bun scripts/migrate-bun.ts

# Display sccache compilation cache hit statistics
sccache-stats:
    sccache --show-stats

# Zero sccache statistics
sccache-zero:
    sccache --zero-stats

# Sync GitHub fork status via `gh` CLI
gh-status:
    gh repo view yolo-rs/yolo-ui --json nameWithOwner,defaultBranchRef,diskUsage,pushedAt
