# AGENTS.md — yolo-ui (Autonomous AI Operations)

## 1. Role & Identity
This repository is the canonical sovereign pillar **yolo-ui** in the YOLO Fleet.
- Package Manager: \un@1.4.4\
- Toolchains: Rust 2024 (edition 2024, resolver 2/3), Bun v1.4.4+, Python uv

## 2. Standard Development Workflows
- **Check**: \un check\ or \cargo check --quiet\
- **Test**: \un test\ or \cargo test --quiet\
- **Lint & Format**: \un lint && bun fmt\
- **Slice Execution**: \un run C:/src/yolo/scripts/slice.ts run <slice-name>\

## 3. Anti-Context Saturation Rules
- Never load all workspace crates simultaneously if not required.
- Query symbols via local AST index or run targeted slices.
- Strictly adhere to NHITL protocol: no emojis, concise factual logs.
