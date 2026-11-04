# wasm-splitter

`wasm-splitter` splits a WebAssembly binary into lazily loaded chunks. Its
runtime, derive macro, CLI, and used-section helper are direct members of the
root Cargo workspace:

- `packages/wasm-splitter`
- `packages/wasm-split-macro`
- `packages/wasm-split-cli`
- `packages/wasm-used`

The test harnesses for these packages live with the workspace tests. They do
not form a nested Cargo workspace.
