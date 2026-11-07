// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  Button,
  Card,
  Dialog,
  FormField,
  Input,
  OMARCHY_SUBTHEME_IDS,
  Select,
  SpringMotion,
  Switch,
  Tabs,
  Toast,
  YOLO_PRESET_IDS,
  buildYoloReactApp,
  createYoloExpoConfig,
  createYoloMetroFacetpackConfig,
  createYoloReactConfig,
  createYoloReactServer,
  createYoloTheme,
  encodeRscFlightStream,
  parseYoloPreset,
  purgeNodeFromPackageJson,
  renderYoloSsrHtml,
} from "../src/index.ts";
import {
  getYoloReactStatus,
  patchExpoDirectory,
  purgeNodeInDirectory,
  scaffoldYoloReactApp,
} from "../../../scripts/react-bun.ts";

describe("@yolo-rs/react — Core 7-Preset Theme Engine", () => {
  test("defaults to neutral ('yolo-neutral') and resolves all 8 built-in presets + 23 Omarchy subthemes", () => {
    expect(YOLO_PRESET_IDS).toEqual([
      "neutral",
      "m3",
      "fluent",
      "apple",
      "omarchy",
      "hyprland",
      "aphrody",
      "chromium",
    ]);
    expect(OMARCHY_SUBTHEME_IDS.length).toBe(23);

    const defaultCfg = createYoloReactConfig();
    expect(defaultCfg.preset).toBe("neutral");
    expect(defaultCfg.enableByronLovePet).toBe(false);

    for (const preset of YOLO_PRESET_IDS) {
      const darkTheme = createYoloTheme(preset, true);
      const lightTheme = createYoloTheme(preset, false);
      expect(darkTheme.preset).toBe(preset);
      expect(darkTheme.canonicalId).toBe(`yolo-${preset}`);
      expect(darkTheme.cssText).toContain(
        `--yolo-theme-preset: "yolo-${preset}";`,
      );
      expect(darkTheme.cssText).toContain("--md-sys-color-primary:");
      expect(darkTheme.reactNativeStyleSheet.container.backgroundColor).toBe(
        darkTheme.tokens.bgPrimary,
      );
      expect(lightTheme.dark).toBe(false);
    }

    const hypr = createYoloTheme("hyprland", true);
    expect(hypr.cssVariables["--yolo-hyprland-active-border"]).toContain(
      "#33ccff",
    );
    expect(hypr.cssVariables["--yolo-hyprland-active-border"]).toContain(
      "#00ff99",
    );

    const aphrodyCfg = createYoloReactConfig({ preset: "aphrody" });
    expect(aphrodyCfg.enableByronLovePet).toBe(true);
    expect(parseYoloPreset("byron-love")).toBe("aphrody");
  });
});

describe("@yolo-rs/react — Universal Primitives (Web DOM + React Native / Expo)", () => {
  test("Button, Card, Dialog, Input, Select, Switch, Tabs, Toast, FormField, and SpringMotion emit Web DOM + React Native props", () => {
    const btn = Button({
      variant: "filled",
      size: "lg",
      label: "Deploy",
      theme: "m3",
    });
    expect(btn.domProps.role).toBe("button");
    expect(btn.domProps["data-yolo-preset"]).toBe("m3");
    expect(btn.nativeProps.accessibilityRole).toBe("button");
    expect(btn.nativeProps.style.height).toBe(40);

    const card = Card({
      variant: "elevated",
      interactive: true,
      theme: "apple",
    });
    expect(card.domProps.role).toBe("button");
    expect(card.nativeProps.accessibilityRole).toBe("button");

    const dialog = Dialog({
      open: true,
      title: "Confirm Action",
      theme: "fluent",
    });
    expect(dialog.domProps.role).toBe("dialog");
    expect(dialog.domProps["aria-expanded"]).toBe(true);
    expect(dialog.nativeProps.accessibilityRole).toBe("alert");

    const input = Input({
      value: "hello",
      invalid: true,
      placeholder: "Email",
    });
    expect(input.domProps["aria-invalid"]).toBe(true);
    expect(input.nativeProps.accessibilityRole).toBe("text");

    const select = Select({ options: ["Rust", "Bun", "Expo"], open: true });
    expect(select.state.value).toBe("Rust");
    expect(select.domProps["aria-expanded"]).toBe(true);

    const sw = Switch({
      checked: true,
      label: "Enable Kawase Blur",
      theme: "hyprland",
    });
    expect(sw.domProps["aria-checked"]).toBe(true);
    expect(sw.nativeProps.accessibilityState.checked).toBe(true);

    const tabs = Tabs({
      tabs: ["Overview", "Metrics", "Logs"],
      activeTab: "Metrics",
    });
    expect(tabs.state.activeIndex).toBe(1);
    expect(tabs.nativeProps.accessibilityRole).toBe("tablist");

    const toast = Toast({ message: "Build succeeded", tone: "success" });
    expect(toast.domProps.role).toBe("status");
    expect(toast.nativeProps.accessibilityLabel).toBe("Build succeeded");

    const field = FormField({
      name: "email",
      label: "Email",
      required: true,
      value: "",
    });
    expect(field.state.invalid).toBe(true);
    expect(field.domProps["aria-invalid"]).toBe(true);

    const motion = SpringMotion("fast", "spatial", 0.16);
    expect(motion.spec.stiffness).toBe(400);
    expect(motion.reanimatedConfig.damping).toBe(24);
    expect(motion.progressAtT).toBeGreaterThan(0.5);
  });
});

describe("@yolo-rs/react — Server (RSC Flight + SSR) & Expo / Node Purge", () => {
  test("createYoloReactServer serves RSC Flight stream, theme CSS, and SSR HTML", async () => {
    const srv = createYoloReactServer({
      appId: "org.yolo.test",
      title: "Yolo RSC Test",
      preset: "omarchy",
      dark: true,
    });

    const cssRes = await srv.handleRequest(
      new Request("http://localhost/_yolo/theme.css?preset=hyprland"),
    );
    expect(cssRes.status).toBe(200);
    expect(await cssRes.text()).toContain("yolo-hyprland");

    const rscRes = await srv.handleRequest(
      new Request("http://localhost/_yolo/rsc"),
    );
    expect(rscRes.status).toBe(200);
    expect(rscRes.headers.get("x-yolo-engine")).toBe("bun+rust-rsc");
    const rscText = await rscRes.text();
    expect(rscText).toContain('0:["$","main"');

    const htmlRes = await srv.handleRequest(new Request("http://localhost/"));
    const html = await htmlRes.text();
    expect(html).toContain("__YOLO_RSC_FLIGHT__");
    expect(html).toContain("Yolo RSC Test");

    const manualFlight = encodeRscFlightStream([
      { id: 0, kind: "model", payload: ["$", "div", null, { children: "ok" }] },
      { id: 1, kind: "client-ref", payload: ["./Client.tsx", "Client"] },
    ]);
    expect(manualFlight).toContain('0:["$');
    expect(manualFlight).toContain("1:I[");
    expect(
      renderYoloSsrHtml({
        title: "Manual",
        preset: "neutral",
        dark: true,
        bodyHtml: "<p>Hi</p>",
        rscFlightPayload: manualFlight,
      }),
    ).toContain('data-yolo-preset="yolo-neutral"');
  });

  test("purgeNodeFromPackageJson and Expo Facetpack OXC configurator work deterministically", () => {
    const purged = purgeNodeFromPackageJson({
      name: "legacy-react-app",
      scripts: {
        start: "npm run dev",
        dev: "npx expo start",
        lint: "yarn run eslint",
        build: "node scripts/build.js",
        typecheck: "pnpm run tsc",
      },
      volta: { node: "20.0.0" },
    });

    expect(purged.purgedPkg.packageManager).toBe("bun@1.4.4");
    expect((purged.purgedPkg.scripts as Record<string, string>).start).toBe(
      "bun run dev",
    );
    expect((purged.purgedPkg.scripts as Record<string, string>).dev).toBe(
      "bunx --bun expo start",
    );
    expect((purged.purgedPkg.scripts as Record<string, string>).build).toBe(
      "bun scripts/build.js",
    );
    expect(purged.removedFields).toContain("volta");

    const expoCfg = createYoloExpoConfig({
      name: "YoloMobile",
      slug: "yolo-mobile",
      preset: "aphrody",
    });
    expect(expoCfg.expo.newArchEnabled).toBe(true);
    expect(expoCfg.expo.extra.facetpackOxc).toBe(true);
    expect(expoCfg.expo.extra.yoloThemePreset).toBe("yolo-aphrody");

    const metroTs = createYoloMetroFacetpackConfig("m3");
    expect(metroTs).toContain("@ecrindigital/facetpack");
    expect(metroTs).toContain("yolo-m3");
  });

  test("scripts/react-bun.ts scaffolds, patches Expo, purges Node, builds with Bun.build, and reports status", async () => {
    const tmp = await mkdtemp(join(tmpdir(), "yolo-react-test-"));
    try {
      const created = await scaffoldYoloReactApp({
        targetDir: join(tmp, "demo-app"),
        template: "universal",
        preset: "hyprland",
      });
      expect(created.preset).toBe("hyprland");
      expect(created.createdFiles).toContain("metro.config.ts");

      const patched = await patchExpoDirectory(
        join(tmp, "demo-app"),
        "omarchy",
      );
      expect(patched.preset).toBe("omarchy");
      expect(await Bun.file(patched.turboModuleDtsPath).exists()).toBe(true);

      await Bun.write(join(tmp, "demo-app/package-lock.json"), "{}");
      const purgeRes = await purgeNodeInDirectory(join(tmp, "demo-app"));
      expect(purgeRes.removedLockfiles).toContain("package-lock.json");

      const buildRes = await buildYoloReactApp({
        entrypoints: [join(import.meta.dir, "../src/index.ts")],
        outdir: join(tmp, "dist"),
        preset: "aphrody",
      });
      expect(buildRes.success).toBe(true);
      expect(await Bun.file(buildRes.themeCssPath).exists()).toBe(true);
      expect(await Bun.file(buildRes.themeRnJsonPath).exists()).toBe(true);

      const status = await getYoloReactStatus();
      expect(status.docsPresent).toBe(true);
      expect(status.packagesPresent.yoloReactPkg).toBe(true);
    } finally {
      await rm(tmp, { recursive: true, force: true });
    }
  });
});
