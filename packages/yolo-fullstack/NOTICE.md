# Attribution

`src/app/` (the App Router runtime: build, scan, plugin, host, dev, prerender, compile, cli, boundaries,
headers, navigation, link, cache, server, image-size and `runtime/*`) is migrated from the
`@aphrody/next-bun/app` entry point of Bun UI.

| Field | Value |
| --- | --- |
| Upstream repository | https://github.com/aphrody-labs/bun-ui |
| Upstream commit | `f2443832fd381fbf8e2d76dcaf6e92ab2f19e107` (2026-10-22, "release: bump all packages, plugins and crates to 1.4.7") |
| Source path | `packages/bun-next/app/**` |
| License | Apache-2.0 (per-file `SPDX-License-Identifier: Apache-2.0`), package `MIT AND Apache-2.0` |
| Consumer | `@yolo-rs/fullstack` (`src/app/`), Bun UI `m3` App Router |
| Migrated on | 2026-11-04 |

Changes made during the migration: the `m3` / `@aphrody/next-bun/app` identifiers, asset prefix (`/_yolo/`),
action header (`yolo-action`), error digests (`YOLO_*`), cache directory (`.yolo`) and the Docker base image
(`oven/bun:1.4-alpine`) were renamed; `next.config`, Turbopack config, `next/font`, the image optimizer, asset
prefix (CDN) support, `next/dynamic`, `next/script`, `next/constants` and `next/config` were added on top.

Everything outside `src/app/` is new code of `yolo-rs/yolo-ui`, licensed `MIT OR Apache-2.0`.
