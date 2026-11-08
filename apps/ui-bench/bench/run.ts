// Drives a real Chromium through the DevTools Protocol and measures every (kit, lane, page):
// LCP, CLS, FCP, TTFB, TBT and long tasks, one real click (Event Timing, INP proxy), and the
// bytes transferred per resource type. Slow 4G + 4x CPU throttling, fresh browser context per
// load (no cache), N runs, median. Usage:
//   bun bench/run.ts --chrome /path/chrome-headless-shell --bun http://127.0.0.1:4100 \
//     --leptos http://127.0.0.1:4200 --runs 5 --out results
import { mkdirSync } from "node:fs";
import os from "node:os";
import { CDP } from "./cdp";

const args = new Map<string, string>();
for (let i = 2; i < Bun.argv.length; i += 2) args.set(Bun.argv[i]!.replace(/^--/, ""), Bun.argv[i + 1]!);

const CHROME = args.get("chrome") ?? process.env.CHROME ?? "chrome-headless-shell";
const LANES: Record<string, string> = {
  bun: args.get("bun") ?? "http://127.0.0.1:4100",
  leptos: args.get("leptos") ?? "http://127.0.0.1:4200",
};
const RUNS = Number(args.get("runs") ?? 5);
const OUT = args.get("out") ?? "results";
const PORT = Number(args.get("port") ?? 9333);
const KITS = (args.get("kits") ?? "rosegriffon,shenron").split(",");
const PAGES: Record<string, string> = { home: "", gallery: "gallery", legal: "legal" };

// Lighthouse "slow 4G" with DevTools throttling: 150 ms RTT x 3.75, 1.6 Mbit/s x 0.9 down,
// 750 kbit/s x 0.9 up (lighthouse-core/config/constants.js).
const NETWORK = {
  offline: false,
  latency: 562.5,
  downloadThroughput: (1474.56 * 1024) / 8,
  uploadThroughput: (675 * 1024) / 8,
  connectionType: "cellular4g",
};
const CPU_RATE = 4;
const VIEWPORT = { width: 412, height: 823, deviceScaleFactor: 1.75, mobile: true };

const OBSERVERS = `(() => {
  const m = (window.__bench = { fcp: null, lcp: null, lcpTag: null, lcpSize: 0, shifts: [], longtasks: [], events: [] });
  const obs = (type, cb, extra) => { try { new PerformanceObserver(l => l.getEntries().forEach(cb)).observe({ type, buffered: true, ...extra }); } catch {} };
  obs("paint", e => { if (e.name === "first-contentful-paint") m.fcp = e.startTime; });
  obs("largest-contentful-paint", e => { m.lcp = e.startTime; m.lcpSize = e.size; m.lcpTag = e.element ? e.element.tagName.toLowerCase() : e.url || null; });
  obs("layout-shift", e => { if (!e.hadRecentInput) m.shifts.push([e.startTime, e.value]); });
  obs("longtask", e => m.longtasks.push([e.startTime, e.duration]));
  obs("event", e => m.events.push({ name: e.name, id: e.interactionId, start: e.startTime, dur: e.duration }), { durationThreshold: 16 });
})();`;

const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));

function median(xs: number[]): number | null {
  const v = xs.filter(x => Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const mid = v.length >> 1;
  return v.length % 2 ? v[mid]! : (v[mid - 1]! + v[mid]!) / 2;
}

/** CLS as defined by web-vitals: largest session window (gap < 1 s, window < 5 s). */
function cls(shifts: [number, number][]): number {
  let best = 0, cur = 0, start = -Infinity, last = -Infinity;
  for (const [t, v] of shifts) {
    if (t - last > 1000 || t - start > 5000) { cur = 0; start = t; }
    cur += v; last = t; best = Math.max(best, cur);
  }
  return best;
}

async function launch() {
  const dir = `${os.tmpdir()}/ui-bench-chrome-${process.pid}`;
  const flags = [
    `--remote-debugging-port=${PORT}`, `--user-data-dir=${dir}`, "--no-first-run", "--no-default-browser-check",
    "--disable-background-networking", "--disable-component-update", "--disable-default-apps", "--disable-sync",
    "--disable-extensions", "--mute-audio", "--password-store=basic", "--use-mock-keychain",
    ...(process.env.CHROME_NO_SANDBOX === "1" ? ["--no-sandbox"] : []),
    "about:blank",
  ];
  const proc = Bun.spawn([CHROME, ...flags], { stdout: "ignore", stderr: "ignore" });
  for (let i = 0; i < 100; i++) {
    try {
      const v = await (await fetch(`http://127.0.0.1:${PORT}/json/version`)).json();
      return { proc, version: v as { Browser: string; "User-Agent": string; webSocketDebuggerUrl: string }, dir };
    } catch { await sleep(100); }
  }
  proc.kill();
  throw new Error(`Chromium did not expose CDP on port ${PORT}`);
}

async function measure(cdp: CDP, url: string) {
  const { browserContextId } = await cdp.send("Target.createBrowserContext", { disposeOnDetach: true });
  const { targetId } = await cdp.send("Target.createTarget", { url: "about:blank", browserContextId });
  const { sessionId } = await cdp.send("Target.attachToTarget", { targetId, flatten: true });
  const s = (m: string, p: Record<string, unknown> = {}) => cdp.send(m, p, sessionId);

  const requests = new Map<string, { url: string; type: string; status: number; bytes: number; done: boolean }>();
  let lastNet = performance.now();
  const off = [
    cdp.on("Network.requestWillBeSent", (p, sid) => {
      if (sid !== sessionId) return;
      requests.set(p.requestId, { url: p.request.url, type: p.type ?? "Other", status: 0, bytes: 0, done: false });
      lastNet = performance.now();
    }),
    cdp.on("Network.responseReceived", (p, sid) => {
      if (sid !== sessionId) return;
      const r = requests.get(p.requestId);
      if (r) { r.type = p.type ?? r.type; r.status = p.response.status; }
    }),
    cdp.on("Network.loadingFinished", (p, sid) => {
      if (sid !== sessionId) return;
      const r = requests.get(p.requestId);
      if (r) { r.bytes = p.encodedDataLength; r.done = true; }
      lastNet = performance.now();
    }),
    cdp.on("Network.loadingFailed", (p, sid) => {
      if (sid !== sessionId) return;
      const r = requests.get(p.requestId);
      if (r) r.done = true;
      lastNet = performance.now();
    }),
  ];

  await s("Page.enable");
  await s("Network.enable");
  await s("Runtime.enable");
  await s("Network.setCacheDisabled", { cacheDisabled: true });
  await s("Network.emulateNetworkConditions", NETWORK);
  await s("Emulation.setCPUThrottlingRate", { rate: CPU_RATE });
  await s("Emulation.setDeviceMetricsOverride", VIEWPORT);
  await s("Page.addScriptToEvaluateOnNewDocument", { source: OBSERVERS });

  const loaded = new Promise<void>(resolve => {
    const un = cdp.on("Page.loadEventFired", (_p, sid) => { if (sid === sessionId) { un(); resolve(); } });
  });
  const t0 = performance.now();
  await s("Page.navigate", { url });
  await Promise.race([loaded, sleep(60_000)]);
  const loadWall = performance.now() - t0;
  // Settle: network idle for 2 s and at least 3 s after load (client rendering runs after load).
  const settleStart = performance.now();
  while (performance.now() - settleStart < 30_000) {
    const inflight = [...requests.values()].some(r => !r.done);
    if (!inflight && performance.now() - lastNet > 2000 && performance.now() - settleStart > 3000) break;
    await sleep(100);
  }

  const evalJSON = async (expr: string) =>
    (await s("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true })).result.value;

  const before = await evalJSON(`(() => {
    const n = performance.getEntriesByType("navigation")[0];
    const b = window.__bench;
    return { nav: { responseStart: n.responseStart, requestStart: n.requestStart, dcl: n.domContentLoadedEventEnd, load: n.loadEventEnd },
      fcp: b.fcp, lcp: b.lcp, lcpTag: b.lcpTag, lcpSize: b.lcpSize, longtasks: b.longtasks.slice(),
      domNodes: document.getElementsByTagName("*").length, hasTarget: !!document.querySelector("[data-bench-click]") };
  })()`);

  // One real click through the input pipeline on the page's marked trigger (tab or disclosure).
  let click: { x: number; y: number; hit: boolean } | null = null;
  if (before.hasTarget) {
    click = await evalJSON(`(() => { const el = document.querySelector("[data-bench-click]");
      el.scrollIntoView({ block: "center", behavior: "instant" }); const r = el.getBoundingClientRect();
      const x = r.x + r.width / 2, y = r.y + r.height / 2;
      return { x, y, hit: el.contains(document.elementFromPoint(x, y)) }; })()`);
    await sleep(500);
    await s("Input.dispatchMouseEvent", { type: "mouseMoved", x: click!.x, y: click!.y });
    await s("Input.dispatchMouseEvent", { type: "mousePressed", x: click!.x, y: click!.y, button: "left", buttons: 1, clickCount: 1 });
    await s("Input.dispatchMouseEvent", { type: "mouseReleased", x: click!.x, y: click!.y, button: "left", buttons: 0, clickCount: 1 });
    await sleep(1500);
  }
  const after = await evalJSON(`(() => { const b = window.__bench;
    return { shifts: b.shifts.slice(), events: b.events.slice(), longtasks: b.longtasks.slice(),
      expanded: (() => { const el = document.querySelector("[data-bench-click]"); if (!el) return null;
        const d = el.closest("details"); if (d) return d.open;
        return el.getAttribute("aria-expanded") ?? el.getAttribute("aria-selected") ?? (document.getElementById(el.htmlFor || "")?.checked ?? null); })() };
  })()`);

  for (const u of off) u();
  await cdp.send("Target.disposeBrowserContext", { browserContextId });

  const fcp = before.fcp as number | null;
  const tbtTasks = (before.longtasks as [number, number][]).filter(([t]) => fcp === null || t >= fcp);
  const tbt = tbtTasks.reduce((acc, [, d]) => acc + Math.max(0, d - 50), 0);
  const interactions = (after.events as { id: number; dur: number; name: string }[]).filter(e => e.id > 0);
  const inp = interactions.length ? Math.max(...interactions.map(e => e.dur)) : null;

  const bytes: Record<string, number> = { total: 0, document: 0, script: 0, stylesheet: 0, font: 0, image: 0, other: 0 };
  for (const r of requests.values()) {
    if (!r.done) continue;
    const k = r.type.toLowerCase();
    const key = k in bytes ? k : k === "fetch" || k === "xhr" ? "other" : k === "media" ? "image" : "other";
    bytes[key]! += r.bytes;
    bytes.total! += r.bytes;
  }

  return {
    ttfb: before.nav.responseStart,
    fcp,
    lcp: before.lcp,
    lcpTag: before.lcpTag,
    cls: cls(after.shifts),
    tbt,
    longTasks: tbtTasks.length,
    longTaskMs: tbtTasks.reduce((a, [, d]) => a + d, 0),
    // Whole load, including tasks before FCP (client rendering delays FCP rather than adding TBT).
    longTasksAll: (before.longtasks as [number, number][]).length,
    longTaskMsAll: (before.longtasks as [number, number][]).reduce((a, [, d]) => a + d, 0),
    inp,
    clickEvents: interactions.map(e => ({ name: e.name, dur: e.dur })),
    clickHit: click?.hit ?? false,
    clickEffective: after.expanded === true || after.expanded === "true",
    dcl: before.nav.dcl,
    load: before.nav.load,
    loadWall,
    domNodes: before.domNodes,
    requests: [...requests.values()].map(r => ({ url: r.url.replace(/^https?:\/\/[^/]+/, ""), type: r.type, status: r.status, bytes: r.bytes })),
    bytes,
  };
}

/** Same busy loop unthrottled and at CPU_RATE, and a check that long tasks are observed. */
async function calibrate(cdp: CDP) {
  const out: Record<string, unknown> = {};
  for (const rate of [1, CPU_RATE]) {
    const { browserContextId } = await cdp.send("Target.createBrowserContext", { disposeOnDetach: true });
    const { targetId } = await cdp.send("Target.createTarget", { url: "about:blank", browserContextId });
    const { sessionId } = await cdp.send("Target.attachToTarget", { targetId, flatten: true });
    await cdp.send("Emulation.setCPUThrottlingRate", { rate }, sessionId);
    const r = await cdp.send("Runtime.evaluate", {
      returnByValue: true,
      awaitPromise: true,
      expression: `new Promise(res => {
        const lt = []; new PerformanceObserver(l => l.getEntries().forEach(e => lt.push(e.duration))).observe({ type: "longtask" });
        setTimeout(() => { const t = performance.now(); let x = 0; for (let i = 0; i < 2e7; i++) x += i % 7; const ms = performance.now() - t;
          setTimeout(() => res({ loopMs: ms, longTasks: lt }), 300); }, 50); })`,
    }, sessionId);
    out[`rate${rate}`] = r.result.value;
    await cdp.send("Target.disposeBrowserContext", { browserContextId });
  }
  return out;
}

async function hostInfo() {
  const cpu = (await Bun.file("/proc/cpuinfo").text().catch(() => "")).match(/model name\s*:\s*(.+)/)?.[1] ?? os.cpus()[0]?.model;
  const osRelease = (await Bun.file("/etc/os-release").text().catch(() => "")).match(/PRETTY_NAME="(.+)"/)?.[1];
  return { hostname: os.hostname(), cpu, cores: os.cpus().length, memGiB: +(os.totalmem() / 2 ** 30).toFixed(1), os: osRelease, kernel: os.release() };
}

const { proc, version } = await launch();
const cdp = await CDP.connect(version.webSocketDebuggerUrl);
const startedAt = new Date().toISOString();
const results: any[] = [];
const calibration = await calibrate(cdp);
console.log("cpu calibration", JSON.stringify(calibration));
try {
  // Warm both servers (Bun bundles HTML imports on first request) outside the measured loads.
  for (const kit of KITS) for (const [lane, base] of Object.entries(LANES)) for (const p of Object.values(PAGES)) {
    const res = await fetch(`${base}/${kit}/${p}`);
    if (!res.ok) throw new Error(`${lane} ${kit}/${p}: HTTP ${res.status}`);
    const html = await res.text();
    for (const m of html.matchAll(/(?:src|href)="(\/[^"]+\.(?:js|css))"/g)) await fetch(`${base}${m[1]}`);
  }
  for (let run = 1; run <= RUNS; run++) {
    for (const [page, path] of Object.entries(PAGES)) for (const kit of KITS) for (const [lane, base] of Object.entries(LANES)) {
      const url = `${base}/${kit}/${path}`;
      const m = await measure(cdp, url);
      results.push({ kit, lane, page, run, url, ...m });
      console.log(
        `run ${run} ${kit.padEnd(11)} ${lane.padEnd(6)} ${page.padEnd(7)} ttfb ${m.ttfb.toFixed(0)} fcp ${m.fcp?.toFixed(0)} lcp ${m.lcp?.toFixed(0)} ` +
          `cls ${m.cls.toFixed(4)} tbt ${m.tbt.toFixed(0)} inp ${m.inp ?? "<16"} js ${m.bytes.script} total ${m.bytes.total} click ${m.clickEffective}`,
      );
    }
  }
} finally {
  cdp.close();
  proc.kill();
}

const keys = ["ttfb", "fcp", "lcp", "cls", "tbt", "longTasks", "longTaskMs", "longTasksAll", "longTaskMsAll", "inp", "dcl", "load", "domNodes"] as const;
const summary: any[] = [];
for (const kit of KITS) for (const lane of Object.keys(LANES)) for (const page of Object.keys(PAGES)) {
  const rows = results.filter(r => r.kit === kit && r.lane === lane && r.page === page);
  const med: Record<string, number | null> = {};
  for (const k of keys) med[k] = median(rows.map(r => (k === "inp" ? (r.inp ?? 0) : r[k]) as number));
  for (const b of ["total", "document", "script", "stylesheet", "font", "image", "other"]) med[`bytes_${b}`] = median(rows.map(r => r.bytes[b]));
  summary.push({ kit, lane, page, runs: rows.length, inpBelowThreshold: rows.filter(r => r.inp === null).length, clickEffective: rows.filter(r => r.clickEffective).length, median: med });
}

const meta = {
  date: startedAt,
  host: await hostInfo(),
  chrome: version.Browser,
  userAgent: version["User-Agent"],
  bun: Bun.version,
  lanes: LANES,
  runs: RUNS,
  calibration,
  throttling: { network: NETWORK, cpuRate: CPU_RATE, viewport: VIEWPORT, cache: "disabled, fresh browser context per load" },
};
mkdirSync(OUT, { recursive: true });
await Bun.write(`${OUT}/raw.json`, JSON.stringify({ meta, results }, null, 2));
await Bun.write(`${OUT}/summary.json`, JSON.stringify({ meta, summary }, null, 2));

const f = (v: number | null, d = 0) => (v === null ? "n/a" : v.toFixed(d));
const kb = (v: number | null) => (v === null ? "n/a" : (v / 1024).toFixed(1));
let md = "| Kit | Lane | Page | TTFB ms | FCP ms | LCP ms | CLS | TBT ms | Long-task ms (whole load) | Click INP ms | Total KiB | JS KiB | CSS KiB | HTML KiB |\n|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n";
for (const r of summary) {
  const m = r.median;
  const inp = r.inpBelowThreshold > r.runs / 2 ? "<16" : f(m.inp);
  md += `| ${r.kit} | ${r.lane} | ${r.page} | ${f(m.ttfb)} | ${f(m.fcp)} | ${f(m.lcp)} | ${f(m.cls, 4)} | ${f(m.tbt)} | ${f(m.longTaskMsAll)} | ${inp} | ${kb(m.bytes_total)} | ${kb(m.bytes_script)} | ${kb(m.bytes_stylesheet)} | ${kb(m.bytes_document)} |\n`;
}
await Bun.write(`${OUT}/summary.md`, md);
console.log(`\n${md}`);
