// SPDX-License-Identifier: MIT OR Apache-2.0
import { describe, expect, test } from "bun:test";
import { createDashboardRoutes } from "../src/server.ts";

describe("YOLO & Aphrody GPUI Desktop Dashboard Server", () => {
  const routes = createDashboardRoutes();

  test("1. GET /api/status returns all 9 active YOLO workspaces", async () => {
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

  test("3. GET /api/fleet returns multi-cloud VPS infrastructure", async () => {
    const res = routes["/api/fleet"]();
    expect(res.status).toBe(200);
    const data = (await res.json()) as {
      hosts: Array<{ name: string; region: string; status: string }>;
    };
    expect(data.hosts.length).toBe(3);
    expect(data.hosts.some((h) => h.region === "Frankfurt")).toBe(true);
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
});
