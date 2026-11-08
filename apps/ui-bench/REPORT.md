# UI bench: rg-ui and Shenron UI, Bun full stack against Leptos static SSR

Date 2026-11-05. Raw data in `results/raw.json`, table in `results/summary.md`.

## Setup

- Kits: rosegriffon `rg-ui` (origin/main 8b296e74, Base UI 1.9.0) and Shenron UI (2e465772, Base UI 1.8.0), imported read-only.
- Bun lane: `Bun.serve` with HTML imports, React 19.3.0 rendered in the browser, Tailwind 4.3.3, Bun 1.4.6-canary.1. Client rendering only, because both kits mark their components `"use client"`.
- Leptos lane: Leptos 0.8.23 on Axum 0.8.9. Pages are rendered once at startup and served as static HTML, with no client JavaScript. They are rebuilt in Leptos with the kits' CSS, so structure and colours match but not pixel for pixel.
- Browser: Chrome for Testing `chrome-headless-shell` 157.0.8103.38, driven over the DevTools protocol from Bun (`bench/run.ts`, `bench/cdp.ts`).
- Conditions: slow 4G (562.5 ms latency, 1474.56 kbit/s), 4x CPU throttling (a calibration loop took 58.5 ms unthrottled and 184.8 ms throttled), mobile viewport 412x823, fresh context per load, no cache.
- Host: bunos, 6 vCPU, 11.4 GiB, Ubuntu 26.10. Rust built on vps (rustc 1.99.1, release, LTO). Median of 5 runs, 3 pages (home, gallery, legal) per kit per lane, 60 loads.
- Servers do not compress, so byte counts are uncompressed.

## Results

Median of 5 runs.

| Kit | Lane | Page | LCP ms | CLS | Click INP ms | Total KiB | JS KiB | CSS KiB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| rosegriffon | bun | home | 4056 | 0 | 40 | 466.9 | 284.6 | 181.7 |
| rosegriffon | bun | gallery | 5296 | 0 | 80 | 636.6 | 454.4 | 181.7 |
| rosegriffon | bun | legal | 4012 | 0 | 56 | 464.3 | 282.0 | 181.7 |
| rosegriffon | leptos | home | 1568 | 0 | 24 | 72.2 | 0 | 60.8 |
| rosegriffon | leptos | gallery | 1592 | 0 | 32 | 76.1 | 0 | 60.8 |
| rosegriffon | leptos | legal | 1580 | 0 | 32 | 92.6 | 0 | 60.8 |
| shenron | bun | home | 5252 | 0 | 56 | 664.9 | 294.6 | 369.8 |
| shenron | bun | gallery | 6312 | 0 | 72 | 831.9 | 461.5 | 369.8 |
| shenron | bun | legal | 5124 | 0 | 72 | 655.2 | 284.9 | 369.8 |
| shenron | leptos | home | 2140 | 0 | 24 | 166.3 | 0 | 157.9 |
| shenron | leptos | gallery | 2136 | 0 | 24 | 169.9 | 0 | 157.9 |
| shenron | leptos | legal | 2144 | 0 | 40 | 191.4 | 0 | 157.9 |

TTFB is about 565 ms in every row: it is the emulated latency plus about 3 ms of server time. LCP equals FCP everywhere. TBT is 0 everywhere because in the Bun lane the script work runs before first paint, where TBT does not count; it appears as late FCP and in the whole-load long-task column of `results/summary.md`.

## Reading

- Leptos static SSR reaches LCP 1.6 to 2.1 s with no JavaScript. The Bun client-rendered lane reaches 4.0 to 6.3 s with 282 to 462 KiB of JavaScript.
- Shenron's CSS is large in both lanes (369.8 KiB in Bun, 157.9 KiB in Leptos), mostly `site.css` and `toriyama.css`. Trimming it is the cheapest gain for Shenron.
- CLS is 0 in all 60 loads.

## Limits

- A Bun lane with server rendering and hydration was not measured and would likely narrow the FCP gap.
- Lab numbers from one headless host and one synthetic click per load.
- The Leptos pages are rebuilds, not the kits' own components.

## Reproduce

```sh
bash scripts/setup.sh
bun scripts/build-leptos-css.ts
# on vps, under the heavy lock
CARGO_TARGET_DIR=~/build/target/ui-bench cargo build --release -j 12 -p ui-bench-leptos
# on bunos
bash scripts/serve.sh start
CHROME_NO_SANDBOX=1 bun bench/run.ts --chrome <chrome-headless-shell> --runs 5 --out results
bash scripts/serve.sh stop
```
