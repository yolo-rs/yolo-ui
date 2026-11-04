# Node.js -> Bun v1.4.4+ & `n2b` Migration Report (`docs/BUN-N2B-MIGRATION.md`)

All JavaScript/TypeScript tooling in `aphrody-labs/yolo-ui` has been migrated from Node.js/npm/webpack/eslint/prettier to **Upstream Bun v1.4.4+** using the **`aphrody-mcp` `n2b` (`node2bun` v0.7.0)** engine and `scripts/migrate-bun.ts`.

## Summary of Purged Legacy Tooling & Dependencies

1. **Removed Legacy Lockfiles & Bundler Configs**:
   - Deleted `packages/extension/package-lock.json`
   - Deleted `packages/extension/webpack.config.js`
   - Deleted `packages/extension/.eslintrc.js`
   - Deleted `packages/playwright-tests/package-lock.json`
2. **Eliminated 12 Redundant npm Packages from `packages/extension/package.json`**:
   - Replaced `webpack`, `webpack-cli`, `ts-loader`, `tslib` -> `bun build ./src/main.ts --outdir ./out --target node --external vscode`
   - Replaced `eslint`, `@typescript-eslint/parser`, `@typescript-eslint/eslint-plugin`, `eslint-config-prettier` -> `bun lint ./src`
   - Replaced `prettier` -> `bun fmt ./src`
   - Replaced `cross-env` -> native Bun environment variable syntax (`TEST_VARIABLE=test bun ...`)
   - Bumped `@types/node` (`^18.0.2` -> `^22.15.0`), `@types/vscode` (`^1.68.1` -> `^1.95.0`), and `typescript` (`^4.7.4` -> `^5.8.3`).
3. **Migrated `.devcontainer/Dockerfile` & `.github/actions/prune-cargo-target/lib.js`**:
   - Replaced `npm install -g` / `npx` / `npm uninstall -g` with `bun install` / `bunx` / `bun remove`.
   - Upgraded Node builtin imports to explicit `node:fs` and `node:path` specifiers.
4. **Unified Root Bun Workspaces (`package.json`)**:
   - Configured `"packageManager": "bun@1.4.4"` and `"workspaces": ["packages/extension", "packages/playwright-tests"]`.
   - Verified via `bun scripts/migrate-bun.ts` (`just n2b-migrate`).
