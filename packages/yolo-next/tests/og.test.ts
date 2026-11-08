// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import {
  ImageResponse,
  OG_CACHE_CONTROL,
  defaultOgFont,
  fallbackOgSvg,
  renderOgSvg,
  resvgAvailable,
  satoriAvailable,
} from "../src/og.ts";

const card = createElement(
  "div",
  {
    style: {
      fontSize: 64,
      background: "#09090b",
      color: "#fafafa",
      width: "100%",
      height: "100%",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
    },
  },
  "Yolo Fixture",
);

const PNG_SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];

describe("ImageResponse", () => {
  test("renders a PNG (or SVG when resvg is unavailable) with the Next.js headers", async () => {
    const response = new ImageResponse(card, { width: 200, height: 100 });
    expect(response).toBeInstanceOf(Response);
    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe(OG_CACHE_CONTROL);
    const type = response.headers.get("content-type");
    expect(["image/png", "image/svg+xml"]).toContain(type ?? "");
    const bytes = new Uint8Array(await response.arrayBuffer());
    expect(bytes.byteLength).toBeGreaterThan(0);
    if (type === "image/png") {
      expect([...bytes.subarray(0, 8)]).toEqual(PNG_SIGNATURE);
      expect(resvgAvailable()).toBe(true);
    } else {
      expect(new TextDecoder().decode(bytes)).toContain("<svg");
    }
  });

  test("honours status, statusText and extra headers", async () => {
    const response = new ImageResponse(card, { width: 64, height: 32, status: 201, statusText: "Made", headers: { "x-test": "1" } });
    expect(response.status).toBe(201);
    expect(response.statusText).toBe("Made");
    expect(response.headers.get("x-test")).toBe("1");
    expect((await response.arrayBuffer()).byteLength).toBeGreaterThan(0);
  });

  test("renderOgSvg produces SVG via satori when a font is available, else the text fallback", async () => {
    const { svg, viaSatori } = await renderOgSvg(card, { width: 200, height: 100 });
    expect(svg.startsWith("<svg")).toBe(true);
    expect(typeof satoriAvailable()).toBe("boolean");
    const font = await defaultOgFont();
    if (satoriAvailable() && font) {
      expect(viaSatori).toBe(true);
    } else {
      expect(viaSatori).toBe(false);
      expect(svg).toContain("Yolo Fixture");
    }
    const fallback = fallbackOgSvg(card, 200, 100);
    expect(fallback).toContain('fill="#09090b"');
    expect(fallback).toContain(">Yolo Fixture</text>");
    expect(fallback).toContain('font-size="64"');
  });

  test("explicit fonts are forwarded to satori", async () => {
    const font = await defaultOgFont();
    if (!font || !satoriAvailable()) {
      return;
    }
    const { svg, viaSatori } = await renderOgSvg(card, { width: 120, height: 60, fonts: [font] });
    expect(viaSatori).toBe(true);
    expect(svg).toContain('width="120"');
  });
});
