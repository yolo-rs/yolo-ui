# Node.js to Bun v1.4.4+ and `n2b` migration report (`docs/BUN-N2B-MIGRATION.md`)

All JavaScript/TypeScript tooling in `aphrody-labs/yolo-ui` has moved from Node.js/npm/webpack/eslint/prettier to upstream Bun v1.4.4+, using the `aphrody-mcp` `n2b` (`node2bun` v0.7.0) engine and `scripts/migrate-bun.ts`.

## Removed legacy tooling and dependencies

1. Removed legacy lockfiles and bundler configs:
   - Deleted `packages/extension/package-lock.json`
   - Deleted `packages/extension/webpack.config.js`
   - Deleted `packages/extension/.eslintrc.js`
   - Deleted `packages/playwright-tests/package-lock.json`
2. Removed 12 redundant npm packages from `packages/extension/package.json`:
   - Replaced `webpack`, `webpack-cli`, `ts-loader` and `tslib` with `bun build ./src/main.ts --outdir ./out --target node --external vscode`
   - Replaced `eslint`, `@typescript-eslint/parser`, `@typescript-eslint/eslint-plugin` and `eslint-config-prettier` with `bun lint ./src`
   - Replaced `prettier` with `bun fmt ./src`
   - Replaced `cross-env` with native Bun environment variable syntax (`TEST_VARIABLE=test bun ...`)
   - Bumped `@types/node` (`^18.0.2` to `^22.15.0`), `@types/vscode` (`^1.68.1` to `^1.95.0`) and `typescript` (`^4.7.4` to `^5.8.3`).
3. Migrated `.devcontainer/Dockerfile` and `.github/actions/prune-cargo-target/lib.js`:
   - Replaced `npm install -g` / `npx` / `npm uninstall -g` with `bun install` / `bunx` / `bun remove`.
   - Switched Node builtin imports to explicit `node:fs` and `node:path` specifiers.
4. Set up root Bun workspaces (`package.json`):
   - Configured `"packageManager": "bun@1.4.4"` and `"workspaces": ["packages/extension", "packages/playwright-tests"]`.
   - Verified with `bun scripts/migrate-bun.ts` (`just n2b-migrate`).
