# =============================================================================
# Yolo UI (`aphrody-labs/yolo-ui`) — End-to-End Task Runner (`Justfile`)
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
    cargo clippy -p yolo-shared -p yolo-gpui -p yolo-iced -p yolo-tauri -p yolo-showcase -- -D warnings

# Run fast typecheck and metadata validation across Bun & Cargo
check:
    bun check
    cargo metadata --no-deps --format-version 1 > $null
    cargo check -p yolo-shared -p yolo-gpui -p yolo-iced -p yolo-tauri -p yolo-showcase

# Run all Bun and Rust unit tests
test: test-bun test-rust

# Run Bun v1.4.4+ monorepo test suite
test-bun:
    bun test

# Run Rust unit tests across core and multi-engine crates
test-rust:
    cargo test -p yolo-shared -p yolo-gpui -p yolo-iced -p yolo-tauri -p yolo-showcase

# Build core crates and showcase app
build:
    bun scripts/build.ts

# Build release artifacts
build-release:
    bun scripts/build.ts --release

# Run the unified 4-engine showcase binary (`apps/yolo-showcase`)
showcase:
    cargo run -p yolo-showcase

# Inspect status of all 70 submodules across the 5 upstream ecosystems
fleet-status:
    bun scripts/fleet.ts status

# Shallow-sync all 70 submodules (`--depth 1`)
fleet-sync:
    bun scripts/fleet.ts sync

# Generate `fleet.json` inventory for all 70 submodules
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
    gh repo view aphrody-labs/yolo-ui --json nameWithOwner,defaultBranchRef,diskUsage,pushedAt
