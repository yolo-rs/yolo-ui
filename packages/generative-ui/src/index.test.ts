import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import {
  compileDesignMdToSixTargets,
  computeGoldenSignature,
  createM3ExpressiveShaderManifest,
  formatGoldenHex,
  parseDesignMd,
  verifyAntiSlopInvariants,
  verifyScreenMathGate,
  yoloZig,
} from "./index";

const YOLO_UI_ROOT = join(import.meta.dir, "..", "..", "..");
const DESIGN_MD_PATH = join(YOLO_UI_ROOT, "DESIGN.md");

describe("yolo-design Fullstack Generative UI + Zig 0.17 FFI + DESIGN.md Compiler", () => {
  test("Zig 0.17 FFI symbols evaluate concentric radii, APCA, WCAG piecewise, SDF, and springs", async () => {
    expect(yoloZig.symbols.yolo_concentric_inner_radius(20, 12)).toBe(8);
    expect(yoloZig.symbols.yolo_is_ai_purple_slop(0.55, 0.22, 280)).toBe(true);
    expect(yoloZig.symbols.yolo_is_ai_purple_slop(0.62, 0.19, 38)).toBe(false);

    const wcag = yoloZig.symbols.yolo_wcag_contrast_ratio(0, 0, 0, 1, 1, 1);
    expect(wcag).toBeCloseTo(21.0, 2);

    const sdfInside = yoloZig.symbols.yolo_rounded_rect_sdf_coverage(50, 50, 0, 0, 100, 100, 12);
    expect(sdfInside).toBe(1.0);

    const gate = await verifyScreenMathGate({ outerRadiusPx: 20, paddingPx: 12 });
    expect(gate.concentricInnerRadiusPx).toBe(8);
    expect(Math.abs(gate.apcaBodyContrast)).toBeGreaterThan(60);
    expect(gate.wcagBodyContrast).toBeGreaterThan(4.5);

    const shader = createM3ExpressiveShaderManifest("liquid-glass", 1600, 1.0);
    expect(shader.settledPositionAt180ms).toBeGreaterThan(0.99);
    expect(shader.settledPositionAt180ms).toBeLessThanOrEqual(1.0);
  });

  test("Pinned Golden FNV-1a 64-bit signature is deterministic across strings and buffers", () => {
    const sigA = formatGoldenHex(computeGoldenSignature("yolo-design-v0.8.0"));
    const sigB = formatGoldenHex(computeGoldenSignature("yolo-design-v0.8.0"));
    expect(sigA).toBe(sigB);
    expect(sigA).toMatch(/^0x[0-9a-f]{16}$/);
  });

  test("DESIGN.md hybrid parser + Anti-Slop linter + 6-target compiler succeeds", () => {
    const out = compileDesignMdToSixTargets(DESIGN_MD_PATH);
    expect(out.files.length).toBe(7);
    for (const f of out.files) {
      expect(existsSync(f)).toBe(true);
    }
    expect(out.goldenSignature).toMatch(/^0x[0-9a-f]{16}$/);
  });

  test("Anti-Slop linter rejects AI indigo/purple and missing tabular-nums", () => {
    const badPurple = `---
name: "Slop"
colors:
  primary: "#6366f1"
typography:
  mono:
    fontFamily: "Inter"
    fontSize: "1rem"
    fontFeature: "tnum"
rounded:
  sm: "4px"
spacing:
  sm: "4px"
---
## Overview
## Colors
## Typography
## Layout
## Elevation & Depth
## Shapes
## Components
## Do's and Don'ts
`;
    const parsed = parseDesignMd(badPurple);
    expect(() => verifyAntiSlopInvariants(parsed)).toThrow(/no-ai-purple-slop/);
  });
});
