import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import {
  ICONS_REPO_ROOT,
  localFile,
  readAssetLock,
  runIconsCli,
  yoloIconsCommand,
} from "../scripts/ui-icons";

const ALLOWED = new Set([
  "Apache-2.0",
  "MIT",
  "MIT OR Apache-2.0",
  "BSD-3-Clause",
  "CC-BY-SA-4.0",
]);

describe("bun ui icons", () => {
  test("asset lock passes the licensing gate and every subset file is materialized", async () => {
    const lock = await readAssetLock();
    expect(lock.set.map((s) => s.id)).toEqual([
      "aphrody",
      "material-symbols",
      "fluent",
      "chromium",
      "cosmic",
    ]);
    for (const set of lock.set) {
      expect(ALLOWED.has(set.license)).toBe(true);
      expect(Boolean(set.commit ?? set.version)).toBe(true);
      expect(
        `${set.name} ${set.upstream}`.toLowerCase().includes("sf-symbols"),
      ).toBe(false);
      if (set.fetch !== "local") {
        expect(existsSync(join(ICONS_REPO_ROOT, set.dir, "LICENSE"))).toBe(
          true,
        );
        expect(existsSync(join(ICONS_REPO_ROOT, set.dir, "NOTICE"))).toBe(true);
      }
      for (const name of Object.keys(set.icons)) {
        expect(existsSync(join(ICONS_REPO_ROOT, localFile(set, name)))).toBe(
          true,
        );
      }
    }
  });

  test("the Aphrody mark is pure vector geometry", async () => {
    for (const grid of [24, 48]) {
      const svg = await Bun.file(
        join(
          ICONS_REPO_ROOT,
          `crates/yolo-icons/assets/aphrody/aphrody_mark_${grid}.svg`,
        ),
      ).text();
      expect(svg).toContain(`viewBox="0 0 ${grid} ${grid}"`);
      expect(/<text|<image|<use|href=/.test(svg)).toBe(false);
      expect(svg.match(/<path /g)?.length).toBe(2);
    }
  });

  test("binary resolution honors YOLO_ICONS_BIN", () => {
    const previous = process.env.YOLO_ICONS_BIN;
    process.env.YOLO_ICONS_BIN = "/opt/yolo/bin/yolo-icons";
    try {
      expect(yoloIconsCommand()).toEqual(["/opt/yolo/bin/yolo-icons"]);
    } finally {
      if (previous === undefined) delete process.env.YOLO_ICONS_BIN;
      else process.env.YOLO_ICONS_BIN = previous;
    }
  });

  test("help and unknown subcommands", async () => {
    expect(await runIconsCli(["help"])).toBe(0);
    expect(await runIconsCli(["wiggle"])).toBe(1);
  });
});
