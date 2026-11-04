/** @jsxImportSource hono/jsx */
import { renderToString } from "hono/jsx/dom/server";
import type { CSSProperties } from "hono/jsx";
import yoloThemePresets from "./theme-presets.json";

export type WorkbenchUiConfig = {
  cdpPort: number;
  braveShields: boolean;
  themePreset?: string;
};

export type ThemePresetId = keyof typeof yoloThemePresets.themes;

export type WorkbenchPageState = {
  themePreset?: string;
  notice?: string;
  evaluation?: string;
  telemetry?: string;
};

export const themePresetIds = Object.keys(
  yoloThemePresets.themes,
) as ThemePresetId[];

export function resolveThemePreset(
  raw: string | null | undefined,
): ThemePresetId {
  return raw && Object.hasOwn(yoloThemePresets.themes, raw)
    ? (raw as ThemePresetId)
    : "yolo-chromium";
}

function WorkbenchView({
  config,
  page,
}: {
  config: WorkbenchUiConfig;
  page: WorkbenchPageState;
}) {
  const preset = resolveThemePreset(page.themePreset ?? config.themePreset);
  const theme = yoloThemePresets.themes[preset];
  const color = theme.colors;
  const inputStyle: CSSProperties = {
    boxSizing: "border-box",
    minInlineSize: 0,
    minBlockSize: "2.75rem",
    padding: "0.5rem 0.75rem",
    border: `1px solid ${color.border}`,
    borderRadius: "0.5rem",
    backgroundColor: color.raised,
    color: color.text,
    font: "inherit",
  };
  const buttonStyle: CSSProperties = {
    minBlockSize: "2.75rem",
    padding: "0.5rem 0.875rem",
    border: `1px solid ${color.border}`,
    borderRadius: "0.5rem",
    backgroundColor: color.accent,
    color: color.canvas,
    font: "inherit",
    fontWeight: 650,
    cursor: "pointer",
  };
  const panelStyle: CSSProperties = {
    display: "flex",
    flexDirection: "column",
    gap: "0.75rem",
    minInlineSize: 0,
    padding: "1rem",
    overflow: "auto",
    backgroundColor: color.panel,
    borderInlineStart: `1px solid ${color.border}`,
    color: color.text,
    fontSize: "0.875rem",
  };
  const outputStyle: CSSProperties = {
    boxSizing: "border-box",
    maxInlineSize: "100%",
    margin: 0,
    padding: "0.75rem",
    overflow: "auto",
    border: `1px solid ${color.border}`,
    borderRadius: "0.5rem",
    backgroundColor: color.canvas,
    color: color.mutedText,
    fontFamily: "ui-monospace, monospace",
    fontSize: "0.75rem",
    whiteSpace: "pre-wrap",
    overflowWrap: "anywhere",
  };
  const hiddenLabel: CSSProperties = {
    position: "absolute",
    inlineSize: "1px",
    blockSize: "1px",
    overflow: "hidden",
    clipPath: "inset(50%)",
    whiteSpace: "nowrap",
  };
  const formStyle: CSSProperties = {
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "0.5rem",
    minInlineSize: 0,
  };

  return (
    <div
      style={{
        boxSizing: "border-box",
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minBlockSize: "100dvb",
        blockSize: "100dvb",
        overflow: "hidden",
        backgroundColor: color.canvas,
        color: color.text,
        fontFamily: "system-ui, sans-serif",
      }}
    >
      <a href="#browser-main" style={hiddenLabel}>
        Skip to browser workspace
      </a>
      <header
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: "0.75rem",
          padding: "0.75rem 1rem",
          backgroundColor: color.panel,
          borderBlockEnd: `1px solid ${color.border}`,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.625rem",
            whiteSpace: "nowrap",
          }}
        >
          <h1 style={{ margin: 0, fontSize: "1rem", fontWeight: 750 }}>
            YOLO-WEB
          </h1>
          <span
            style={{
              padding: "0.25rem 0.5rem",
              border: `1px solid ${color.border}`,
              borderRadius: "999px",
              color: color.accentSecondary,
              fontSize: "0.75rem",
              fontWeight: 650,
            }}
          >
            CDP workbench
          </span>
        </div>
        <search style={{ flex: "1 1 32rem", minInlineSize: 0 }}>
          <form action="/navigate" method="post" style={formStyle}>
            <label for="browser-address" style={hiddenLabel}>
              Website address
            </label>
            <input
              id="browser-address"
              name="url"
              type="url"
              value="https://example.com"
              required
              style={{ ...inputStyle, flex: "1 1 18rem" }}
            />
            <input name="theme" type="hidden" value={preset} />
            <button type="submit" style={buttonStyle}>
              Navigate
            </button>
          </form>
        </search>
        <form action="/" method="get" style={formStyle}>
          <label
            for="theme-preset"
            style={{ color: color.mutedText, fontSize: "0.8rem" }}
          >
            Theme
          </label>
          <select
            id="theme-preset"
            name="theme"
            style={{ ...inputStyle, flex: "0 1 auto" }}
          >
            {themePresetIds.map((id) => (
              <option value={id} selected={id === preset}>
                {yoloThemePresets.themes[id].name}
              </option>
            ))}
          </select>
          <button
            type="submit"
            style={{
              ...buttonStyle,
              backgroundColor: color.raised,
              color: color.text,
            }}
          >
            Apply
          </button>
        </form>
      </header>
      <main
        id="browser-main"
        style={{
          display: "grid",
          gridTemplateColumns:
            "repeat(auto-fit, minmax(min(100%, 27rem), 1fr))",
          minBlockSize: 0,
          overflow: "auto",
        }}
      >
        <section
          aria-label="Connected browser viewport"
          style={{
            display: "grid",
            placeItems: "center",
            minBlockSize: "18rem",
            overflow: "auto",
            backgroundColor: color.canvas,
          }}
        >
          <img
            src="/stream"
            alt="Live screenshot from the connected Chromium browser"
            style={{
              display: "block",
              maxInlineSize: "100%",
              maxBlockSize: "100%",
              aspectRatio: "16 / 9",
              objectFit: "contain",
            }}
          />
        </section>
        <aside aria-label="Browser tools and telemetry" style={panelStyle}>
          <h2 style={{ margin: 0, fontSize: "1rem" }}>Browser connection</h2>
          <p style={{ margin: 0, color: color.mutedText }}>
            {page.notice ??
              "Page rendering is provided by the connected external CDP browser."}
          </p>
          <pre tabIndex={0} style={outputStyle}>
            {page.telemetry ??
              `CDP :${config.cdpPort} · theme ${theme.name} · waiting for a viewer connection`}
          </pre>
          <h2 style={{ margin: "0.25rem 0 0", fontSize: "1rem" }}>
            JavaScript evaluation
          </h2>
          <form action="/evaluate" method="post" style={formStyle}>
            <label for="javascript-expression" style={hiddenLabel}>
              JavaScript expression to evaluate
            </label>
            <input
              id="javascript-expression"
              name="expression"
              type="text"
              placeholder="document.title"
              style={{ ...inputStyle, flex: "1 1 14rem" }}
            />
            <input name="theme" type="hidden" value={preset} />
            <button type="submit" style={buttonStyle}>
              Evaluate in browser
            </button>
          </form>
          <pre
            tabIndex={0}
            aria-live="polite"
            aria-atomic="true"
            style={outputStyle}
          >
            {page.evaluation ?? "Ready."}
          </pre>
        </aside>
      </main>
      <footer
        style={{
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "space-between",
          gap: "0.5rem",
          padding: "0.75rem 1rem",
          borderBlockStart: `1px solid ${color.border}`,
          backgroundColor: color.panel,
          color: color.mutedText,
          fontFamily: "ui-monospace, monospace",
          fontSize: "0.75rem",
        }}
      >
        <span role="status">
          {page.notice ? "Browser action complete" : "Browser workbench ready"}
        </span>
        <span>
          CDP :{config.cdpPort} · query sanitization{" "}
          {config.braveShields ? "on" : "off"} · {preset}
        </span>
      </footer>
    </div>
  );
}

export function renderHumanBrowserHtml(
  config: WorkbenchUiConfig,
  page: WorkbenchPageState = {},
): string {
  const preset = resolveThemePreset(page.themePreset ?? config.themePreset);
  const renderConfig = { ...config, themePreset: preset };
  const renderPage = { ...page, themePreset: preset };
  return `<!doctype html>${renderToString(
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="color-scheme" content="dark light" />
        <meta
          name="description"
          content="A Bun-hosted browser research workbench for controlling Chromium over the Chrome DevTools Protocol."
        />
        <title>YOLO-Web Browser Workbench</title>
      </head>
      <body style={{ margin: 0 }}>
        <WorkbenchView config={renderConfig} page={renderPage} />
      </body>
    </html>,
  )}`;
}
