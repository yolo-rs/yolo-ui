// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import {
  SHIM_MAP,
  SHIMS_DIR,
  clientReferenceStub,
  parseJsonc,
  resolveNextSpecifier,
  resolveSelfSpecifier,
  resolveTsconfigPath,
  serverReferenceId,
  serverReferenceStub,
  shimSpecifierFor,
  transformModule,
  tsconfigPaths,
  yoloNextPlugin,
} from "../src/plugin.ts";
import { detectDirective } from "../src/segments.ts";
import type { ModuleDirective } from "../src/types.ts";
import { FIXTURE, fakeBuild } from "./helpers.ts";

const posix = (path: string): string => path.replace(/\\/g, "/");
const counterSource = readFileSync(join(FIXTURE, "app", "components", "Counter.tsx"), "utf8");
const actionsSource = readFileSync(join(FIXTURE, "app", "actions.ts"), "utf8");
const pageSource = readFileSync(join(FIXTURE, "app", "page.tsx"), "utf8");

describe("directive detection", () => {
  test("table", () => {
    const cases: Array<[string, ModuleDirective]> = [
      ['"use client";\nexport const a = 1;', "use-client"],
      ["'use client'\nexport const a = 1;", "use-client"],
      ['// leading comment\n"use client";', "use-client"],
      ["/* block\ncomment */ 'use server';", "use-server"],
      ['"use strict";\n"use client"\nexport {};', "use-client"],
      ['#!/usr/bin/env bun\n"use server";', "use-server"],
      ['"use cache";', "use-cache"],
      ['"use cache: private";\nexport async function f() {}', "use-cache"],
      ['import x from "y";\n"use client";', "none"],
      ['const a = "use client";', "none"],
      ['"use client" + x;', "none"],
      ["export default function Page() { return null }", "none"],
      ["", "none"],
    ];
    for (const [source, expected] of cases) expect(detectDirective(source)).toBe(expected);
  });
});

describe("transformModule", () => {
  test("rsc + use client -> registerClientReference stub", () => {
    const result = transformModule(counterSource, "app/components/Counter.tsx", "rsc");
    expect(result.directive).toBe("use-client");
    expect(result.clientModule).toBe(true);
    expect(result.loader).toBe("ts");
    expect(result.contents).toContain('import { registerClientReference } from "react-server-dom-webpack/server.edge";');
    expect(result.contents).toContain('export const Counter = registerClientReference(function() { throw new Error(');
    expect(result.contents).toContain('"app/components/Counter.tsx", "Counter")');
    expect(result.contents).not.toContain("useState");
    const stub = clientReferenceStub("x.tsx", { names: ["A"], hasDefault: true });
    expect(stub).toContain('export default registerClientReference(');
    expect(stub).toContain('"x.tsx", "default")');
  });

  test("client/server + use server -> createServerReference stub with 40-hex ids", () => {
    for (const env of ["client", "server"] as const) {
      const result = transformModule(actionsSource, "app/actions.ts", env);
      expect(result.directive).toBe("use-server");
      expect(result.serverActions).toEqual(["createPost", "deletePost"]);
      expect(result.contents).toContain('import { createServerReference } from "react-server-dom-webpack/client";');
      expect(result.contents).toContain('import { callServer } from "@yolo-rs/next/actions";');
      const ids = [...(result.contents ?? "").matchAll(/createServerReference\("([0-9a-f]+)"/g)].map((m) => m[1]);
      expect(ids).toHaveLength(2);
      for (const id of ids) expect(id).toMatch(/^00[0-9a-f]{38}$/);
      expect(ids[0]).toBe(serverReferenceId("app/actions.ts", "createPost"));
      expect(result.contents).toContain('undefined, undefined, "createPost");');
    }
    expect(serverReferenceStub("a.ts", { names: ["f"], hasDefault: true }, "./my-actions.ts")).toContain('import { callServer } from "./my-actions.ts";');
  });

  test("rsc + use server -> source plus registerServerReference trailer", () => {
    const result = transformModule(actionsSource, "app/actions.ts", "rsc");
    expect(result.contents?.startsWith(actionsSource)).toBe(true);
    expect(result.contents).toContain('from "react-server-dom-webpack/server.edge"');
    expect(result.contents).toContain(`__yoloRegisterServerReference(createPost, "${serverReferenceId("app/actions.ts", "createPost")}", "createPost")`);
    expect(result.loader).toBe("ts");
  });

  test("modules without a matching directive fall through", () => {
    expect(transformModule(counterSource, "app/components/Counter.tsx", "client").contents).toBeUndefined();
    expect(transformModule(counterSource, "app/components/Counter.tsx", "server").contents).toBeUndefined();
    expect(transformModule(pageSource, "app/page.tsx", "rsc").contents).toBeUndefined();
    expect(transformModule(pageSource, "app/page.tsx", "rsc").loader).toBe("tsx");
  });

  test("serverReferenceId is deterministic and salted", () => {
    expect(serverReferenceId("app/actions.ts", "createPost")).toBe(serverReferenceId("app/actions.ts", "createPost"));
    expect(serverReferenceId("app/actions.ts", "createPost")).not.toBe(serverReferenceId("app/actions.ts", "deletePost"));
    expect(serverReferenceId("app/actions.ts", "createPost", "salt")).not.toBe(serverReferenceId("app/actions.ts", "createPost"));
    expect(serverReferenceId("a", "b")).toHaveLength(40);
  });
});

describe("next/* resolution", () => {
  test("every shim specifier maps to a file under src/shims", () => {
    for (const [specifier, file] of Object.entries(SHIM_MAP)) {
      const resolved = resolveNextSpecifier(specifier);
      expect(resolved).toBeDefined();
      expect(posix(resolved ?? "")).toBe(posix(join(SHIMS_DIR, file)));
      expect(shimSpecifierFor(specifier)).toBe(`@yolo-rs/next/shims/${file.replace(/\.ts$/, "")}`);
    }
    expect(posix(resolveNextSpecifier("next/link.js") ?? "")).toBe(posix(join(SHIMS_DIR, "link.ts")));
    expect(posix(resolveNextSpecifier("next/font/google") ?? "")).toBe(posix(join(SHIMS_DIR, "font-google.ts")));
    expect(resolveNextSpecifier("next")).toBeUndefined();
    expect(resolveNextSpecifier("next/dist/server/x")).toBeUndefined();
    expect(posix(resolveSelfSpecifier("@yolo-rs/next/plugin") ?? "")).toBe(posix(join(SHIMS_DIR, "..", "plugin.ts")).replace("/shims/../", "/"));
  });

  test("the plugin resolves next/*, next (empty) and tsconfig paths", async () => {
    const plugin = yoloNextPlugin({ root: FIXTURE, env: "server" });
    const build = fakeBuild("bun");
    plugin.setup(build.build);
    expect(posix((await build.resolve("next/navigation"))?.path ?? "")).toBe(posix(join(SHIMS_DIR, "navigation.ts")));
    expect(posix((await build.resolve("server-only"))?.path ?? "")).toBe(posix(join(SHIMS_DIR, "server-only.ts")));
    expect(await build.resolve("next")).toEqual({ path: "next", namespace: "yolo-empty" });
    const empty = await build.load("next", "yolo-empty");
    expect(empty && "contents" in empty ? empty.contents : "").toBe("export default {};\n");
    expect(posix((await build.resolve("@/public/hero.png"))?.path ?? "")).toBe(posix(join(FIXTURE, "public", "hero.png")));
    expect(posix((await build.resolve("@/app/lib/data"))?.path ?? "")).toBe(posix(join(FIXTURE, "app", "lib", "data.ts")));
    expect(await build.resolve("@/does/not/exist")).toBeUndefined();
    expect(await build.resolve("react")).toBeUndefined();
  });

  test("onLoad applies the directive transform under the root only", async () => {
    const seen: string[] = [];
    const plugin = yoloNextPlugin({ root: FIXTURE, env: "rsc", onClientModule: (id) => seen.push(id) });
    const build = fakeBuild("bun");
    plugin.setup(build.build);
    const counter = await build.load(join(FIXTURE, "app", "components", "Counter.tsx"));
    expect(counter && "contents" in counter ? String(counter.contents) : "").toContain("registerClientReference");
    expect(seen).toEqual(["app/components/Counter.tsx"]);
    // Files without a directive pass through unchanged (Bun 1.4.6 rejects `undefined` for real files).
    const page = await build.load(join(FIXTURE, "app", "page.tsx"));
    expect(page).toEqual({ contents: await Bun.file(join(FIXTURE, "app", "page.tsx")).text(), loader: "tsx" });
    expect(await build.load(join(FIXTURE, "..", "outside.tsx"))).toBeUndefined();
    expect(await build.load(join(FIXTURE, "node_modules", "pkg", "index.js"))).toBeUndefined();
  });

  test("tsconfig paths helpers", () => {
    const paths = tsconfigPaths(FIXTURE);
    expect(paths.aliases).toHaveLength(1);
    expect(paths.aliases[0]).toMatchObject({ prefix: "@/", suffix: "", wildcard: true, targets: ["./*"] });
    expect(posix(resolveTsconfigPath("@/proxy", paths) ?? "")).toBe(posix(join(FIXTURE, "proxy.ts")));
    expect(parseJsonc('{\n  // comment\n  "a": [1, 2,],\n  /* block */ "b": "x//y",\n}')).toEqual({ a: [1, 2], b: "x//y" });
  });
});
