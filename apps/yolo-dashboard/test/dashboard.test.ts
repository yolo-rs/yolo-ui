// SPDX-License-Identifier: MIT OR Apache-2.0
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import { createDashboardRoutes } from "../src/server.ts";

describe("YOLO & Aphrody GPUI Desktop Dashboard Server", () => {
  const routes = createDashboardRoutes();

  test("1. GET /api/status returns all 9 active YOLO workspaces on main", async () => {
    const res = routes["/api/status"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      system: string;
      workspaces: Array<{ name: string; branch: string; clean: boolean }>;
    };
    expect(data.system).toContain("YOLO & Aphrody");
    expect(data.workspaces.length).toBe(9);
    expect(data.workspaces.every((w) => w.branch === "main" && w.clean)).toBe(true);
  });

  test("2. GET /api/orgs returns accurate census for aphrody-labs, yolo-rs, aphrody-dev", async () => {
    const res = routes["/api/orgs"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      summary: { totalRepos: number; keepCount: number; archiveCount: number };
    };
    expect(data.summary.totalRepos).toBeGreaterThanOrEqual(400);
    expect(data.summary.keepCount).toBeGreaterThan(0);
    expect(data.summary.archiveCount).toBeGreaterThan(0);
  });

  test("3. GET /api/fleet returns 3-node WireGuard mesh fleet (prod, vps, bunos) and 4 agents", async () => {
    const res = routes["/api/fleet"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      meshSubnet: string;
      hosts: Array<{ name: string; meshIp: string; status: string; agents: Array<{ name: string; port: number }> }>;
    };
    expect(data.meshSubnet).toBe("10.200.77.0/24");
    expect(data.hosts.length).toBe(3);
    expect(data.hosts.map((h) => h.name)).toEqual(["prod", "vps", "bunos"]);
    expect(data.hosts.every((h) => h.agents.length === 4)).toBe(true);
  });

  test("4. GET /api/registries returns all 5 package ecosystems", async () => {
    const res = routes["/api/registries"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      registries: Array<{ name: string; ready: boolean }>;
    };
    expect(data.registries.length).toBe(5);
    expect(data.registries.every((r) => r.ready)).toBe(true);
  });

  test("5. GET /api/arena returns active multi-agent scoreboard", async () => {
    const res = routes["/api/arena"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      scoreboard: Record<string, number>;
    };
    expect(data.scoreboard.claude).toBeGreaterThan(0);
    expect(data.scoreboard.opencode).toBeGreaterThan(0);
    expect(data.scoreboard.agy).toBeGreaterThan(0);
  });

  test("6. GET /api/design-system returns yolo-aphrody DTCG tokens and WCAG 2.2 AAA contrast matrix", async () => {
    const res = routes["/api/design-system"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      preset: string;
      contrastAudit: Array<{ ratio: number; verdict: string }>;
      checklist: Record<string, unknown>;
    };
    expect(data.preset).toBe("yolo-aphrody");
    expect(data.contrastAudit.length).toBe(4);
    expect(data.contrastAudit.every((c) => c.ratio >= 7.0 && c.verdict === "AAA")).toBe(true);
    expect(data.checklist.zeroEmojis).toBe(true);
    expect(data.checklist.zeroBannedFonts).toBe(true);
  });

  test("7. GET /api/replica returns 100.0% parity across yolo-search and yolo-ai", async () => {
    const res = routes["/api/replica"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      parityMatrices: Array<{ id: string; score: number; mustHaveDone: number; mustHaveTotal: number }>;
    };
    expect(data.parityMatrices.length).toBe(2);
    expect(data.parityMatrices.every((m) => m.score === 100.0 && m.mustHaveDone === m.mustHaveTotal)).toBe(true);
  });

  test("8. dashboard-ui.html complies with Modern Web Guidelines, UI/UX Pro Max, and Zero-Emoji / Zero-Roboto rules", () => {
    const html = readFileSync(join(import.meta.dir, "../src/dashboard-ui.html"), "utf-8");
    const emojis = html.match(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/gu) ?? [];
    expect(emojis.length).toBe(0);
    expect(html.includes("Roboto")).toBe(false);
    expect(/\bInter\b/.test(html)).toBe(false);
    expect(html.includes("Cinzel")).toBe(true);
    expect(html.includes("Plus Jakarta Sans")).toBe(true);
    expect(html.includes("JetBrains Mono")).toBe(true);
    expect(html.includes("color-scheme: dark light")).toBe(true);
    expect(html.includes(":focus-visible")).toBe(true);
    expect(html.includes("forced-colors: active")).toBe(true);
    expect(html.includes("prefers-reduced-motion: reduce")).toBe(true);
    expect(html.includes('role="tablist"')).toBe(true);
  });
});
