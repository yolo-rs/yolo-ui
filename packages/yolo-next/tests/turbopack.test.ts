// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { loadNextConfig } from "../src/config.ts";
import { FIXTURE, fakeBuild } from "./helpers.ts";
import {
  aliasPlugin,
  bundlerPluginsTs,
  bunfigToml,
  globToRegexSource,
  loaderOverridePlugin,
  mapTurbopackLoader,
  markdownPlugin,
  markdownToHtml,
  parseSvgRoot,
  svgReactPlugin,
  textPlugin,
  translateTurbopack,
  yoloBundlerPlugins,
} from "../src/turbopack.ts";

describe("translateTurbopack", () => {
  test("fixture rules, aliases, extensions, define, external", async () => {
    const { config } = await loadNextConfig(FIXTURE);
    const plan = translateTurbopack(config);
    expect(plan.plugins.map((p) => [p.kind, p.filter, p.loader])).toEqual([
      ["svg-react", "\\.svg$", "tsx"],
      ["text", "\\.md$", "text"],
      ["text", "\\.graphql$", "text"],
      ["loader-override", "\\.yaml$", "yaml"],
    ]);
    expect(plan.plugins[0]?.originLoader).toBe("@svgr/webpack");
    expect(plan.plugins[0]?.options).toEqual({ as: "*.js" });
    expect(plan.loaders).toEqual({ ".md": "text", ".graphql": "text", ".yaml": "yaml" });
    expect(plan.aliases).toEqual({ underscore: "lodash", "@/*": "./*" });
    expect(plan.browserAliases).toEqual({ mocha: "mocha/browser-entry.js", fs: "./empty.ts" });
    expect(plan.emptyModules).toEqual([]);
    expect(plan.extensions).toEqual([".mdx", ".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"]);
    expect(plan.define).toEqual({ __BUILD_FLAG__: "true", "process.env.APP_NAME": '"Yolo Fixture"', "process.env.API_BASE": '"https://api.example.com"' });
    expect(plan.external).toEqual(["sharp", "pino"]);
    expect(plan.unsupported).toEqual([]);
    expect(plan.root).toBeDefined();
  });

  test("unsupported loaders, type overrides and false aliases", () => {
    const plan = translateTurbopack({
      turbopack: {
        rules: {
          "*.scss": { loaders: ["sass-loader"], as: "*.css" },
          "*.bin": { type: "bytes" },
          "**/*.mdx": { loaders: [{ loader: "@mdx-js/loader", options: { providerImportSource: "@mdx-js/react" } }], as: "*.js" },
          "*.json5": [{ loaders: ["json5-loader"] }],
        },
        resolveAlias: { canvas: false, lodash: ["lodash-es", "lodash"], "node:fs": { browser: false } },
      },
    });
    expect(plan.plugins.map((p) => p.kind)).toEqual(["unsupported", "loader-override", "markdown", "loader-override"]);
    expect(plan.plugins[1]?.loader).toBe("file");
    expect(plan.plugins[2]?.options?.providerImportSource).toBe("@mdx-js/react");
    expect(plan.plugins[3]?.loader).toBe("jsonc");
    expect(plan.emptyModules).toEqual(["canvas", "node:fs"]);
    expect(plan.aliases).toEqual({ lodash: "lodash-es" });
    expect(plan.unsupported.map((u) => u.support)).toEqual(["unsupported", "partial", "partial", "partial", "partial"]);
    expect(plan.extensions).toEqual([".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"]);
  });

  test("glob and loader mapping helpers", () => {
    expect(globToRegexSource("*.svg")).toBe("\\.svg$");
    expect(globToRegexSource("**/*.md")).toBe("\\.md$");
    expect(globToRegexSource("src/icons/*.svg")).toBe("(?:^|/)src/icons/[^/]*\\.svg$");
    expect(globToRegexSource("app/**/*.graphql")).toBe("(?:^|/)app/(?:.*/)?[^/]*\\.graphql$");
    expect(globToRegexSource("icon-*.svg")).toBe("(?:^|/)icon-[^/]*\\.svg$");
    expect(globToRegexSource("*.{yml,yaml}")).toBe("\\.(?:yml|yaml)$");
    expect(new RegExp(globToRegexSource("src/icons/*.svg")).test("C:/p/src/icons/a.svg")).toBe(true);
    expect(new RegExp(globToRegexSource("src/icons/*.svg")).test("C:/p/src/icons/sub/a.svg")).toBe(false);
    expect(mapTurbopackLoader("raw-loader").kind).toBe("text");
    expect(mapTurbopackLoader("file-loader")).toMatchObject({ kind: "loader-override", loader: "file" });
    expect(mapTurbopackLoader("babel-loader").support).toBe("unsupported");
  });
});

describe("emitters", () => {
  test("bunfigToml", async () => {
    const { config } = await loadNextConfig(FIXTURE);
    const plan = translateTurbopack(config);
    const toml = bunfigToml(plan, { publicEnvPrefixes: ["NEXT_PUBLIC_", "YOLO_PUBLIC_"] });
    expect(toml).toContain('[serve.static]\nplugins = ["./yolo.bundler.ts"]\nenv = "NEXT_PUBLIC_*"\nsourcemap = "linked"');
    expect(toml).toContain("[serve.static.define]\n\"__BUILD_FLAG__\" = \"true\"");
    expect(toml).toContain('[serve.static.loader]\n".md" = "text"');
    expect(toml).toContain("[install]\nexact = true");
    expect(toml).toContain('[test]\nroot = "./tests"');
  });

  test("bundlerPluginsTs", async () => {
    const { config } = await loadNextConfig(FIXTURE);
    const ts = bundlerPluginsTs(translateTurbopack(config));
    expect(ts).toContain('import type { BunPlugin } from "bun";');
    expect(ts).toContain('import { aliasPlugin, loaderOverridePlugin, svgReactPlugin, textPlugin } from "@yolo-rs/next/turbopack";');
    expect(ts).toContain("svgReactPlugin(/\\.svg$/),");
    expect(ts).toContain("textPlugin(/\\.md$/),");
    expect(ts).toContain('loaderOverridePlugin(/\\.yaml$/, "yaml"),');
    expect(ts).toContain('aliasPlugin({"underscore":"lodash","@/*":"./*"}, {"mocha":"mocha/browser-entry.js","fs":"./empty.ts"}, [], import.meta.dir),');
    expect(ts).toContain("export default bundler;");
  });
});

describe("Bun plugin factories", () => {
  test("aliasPlugin resolves exact, browser and empty aliases", async () => {
    const plugin = aliasPlugin({ underscore: "lodash", "@/*": "./*" }, { mocha: "mocha/browser-entry.js", fs: "./empty.ts" }, ["canvas"], FIXTURE);
    const server = fakeBuild("bun");
    plugin.setup(server.build);
    expect((await server.resolve("underscore"))?.path).toMatch(/lodash/);
    expect(await server.resolve("mocha")).toBeUndefined();
    expect((await server.resolve("canvas"))?.namespace).toBe("yolo-empty");
    expect((await server.resolve("@/public/hero.png"))?.path.replace(/\\/g, "/")).toBe(join(FIXTURE, "public", "hero.png").replace(/\\/g, "/"));
    expect(await server.resolve("react")).toBeUndefined();
    const browser = fakeBuild("browser");
    plugin.setup(browser.build);
    expect((await browser.resolve("mocha"))?.path).toMatch(/mocha\/browser-entry\.js$/);
    expect((await browser.resolve("fs"))?.path.replace(/\\/g, "/")).toBe(join(FIXTURE, "empty.ts").replace(/\\/g, "/"));
    const empty = await browser.load("canvas", "yolo-empty");
    expect(empty && "contents" in empty ? empty.contents : "").toBe("export default {};\n");
  });

  test("svgReactPlugin emits a React component module", async () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-svg-"));
    try {
      const file = join(dir, "icon.svg");
      writeFileSync(file, '<?xml version="1.0"?>\n<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" stroke-width="2" class="icon" data-x=\'y\'>\n  <path d="M0 0h24v24H0z"/>\n</svg>\n');
      const parsed = parseSvgRoot(await Bun.file(file).text());
      expect(parsed.attrs).toEqual({ xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 24 24", strokeWidth: "2", className: "icon", "data-x": "y" });
      expect(parsed.inner).toBe('<path d="M0 0h24v24H0z"/>');
      const build = fakeBuild("browser");
      svgReactPlugin(/\.svg$/).setup(build.build);
      const result = await build.load(file);
      const contents = result && "contents" in result ? String(result.contents) : "";
      expect(result && "loader" in result ? result.loader : "").toBe("tsx");
      expect(contents).toContain('createElement("svg"');
      expect(contents).toContain("export default function SvgIcon(props)");
      expect(contents).toContain("export const src = \"data:image/svg+xml,");
      expect(contents).toContain('"viewBox":"0 0 24 24"');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("text, markdown and loader override plugins", async () => {
    const dir = mkdtempSync(join(tmpdir(), "yolo-text-"));
    try {
      const md = join(dir, "doc.md");
      writeFileSync(md, "# Title\n\nSome *text*.\n");
      const yaml = join(dir, "data.yaml");
      writeFileSync(yaml, "a: 1\n");
      const build = fakeBuild("bun");
      textPlugin(/\.md$/).setup(build.build);
      const text = await build.load(md);
      expect(text && "loader" in text ? text.loader : "").toBe("text");
      const mdBuild = fakeBuild("bun");
      markdownPlugin(/\.md$/).setup(mdBuild.build);
      const rendered = await mdBuild.load(md);
      const contents = rendered && "contents" in rendered ? String(rendered.contents) : "";
      expect(contents).toContain("export const html = ");
      expect(contents).toContain("<h1>");
      expect(markdownToHtml("# hi")).toContain("<h1>");
      const override = fakeBuild("bun");
      loaderOverridePlugin(/\.yaml$/, "yaml").setup(override.build);
      const yamlResult = await override.load(yaml);
      expect(yamlResult && "loader" in yamlResult ? yamlResult.loader : "").toBe("yaml");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("yoloBundlerPlugins instantiates one plugin per spec plus the alias plugin", async () => {
    const { config } = await loadNextConfig(FIXTURE);
    const plugins = yoloBundlerPlugins(translateTurbopack(config));
    expect(plugins.map((p) => p.name)).toEqual(["yolo:alias-resolve", "yolo:svg-react", "yolo:text", "yolo:text", "yolo:loader-override:yaml"]);
  });
});
