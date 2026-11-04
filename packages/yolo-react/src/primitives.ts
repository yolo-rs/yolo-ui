// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/react/primitives — Universal Component Contracts & State Machines
// Migrated and unified from @aphrody/m3-react, @aphrody/m3-primitives, @aphrody/m3-forms, and @aphrody/m3-motion.
// Produces both React 19 Web DOM props (ARIA + CSS vars/Tailwind) and React Native / Expo props (accessibilityRole + Yoga/Taffy styles).

import { createYoloTheme, type YoloPresetId, type YoloTheme } from "./core.ts";

export interface UniversalDomProps {
  readonly role?: string;
  readonly className: string;
  readonly style: Record<string, string | number>;
  readonly "data-yolo-component": string;
  readonly "data-yolo-preset": YoloPresetId;
  readonly "aria-disabled"?: boolean;
  readonly "aria-busy"?: boolean;
  readonly "aria-checked"?: boolean;
  readonly "aria-expanded"?: boolean;
  readonly "aria-selected"?: boolean;
  readonly "aria-invalid"?: boolean;
  readonly "aria-describedby"?: string;
  readonly "aria-label"?: string;
  readonly tabIndex?: number;
}

export interface UniversalReactNativeProps {
  readonly accessibilityRole:
    | "button"
    | "link"
    | "switch"
    | "tab"
    | "tablist"
    | "alert"
    | "none"
    | "header"
    | "text"
    | "combobox";
  readonly accessibilityState: {
    readonly disabled?: boolean;
    readonly busy?: boolean;
    readonly checked?: boolean;
    readonly selected?: boolean;
    readonly expanded?: boolean;
  };
  readonly accessibilityLabel?: string;
  readonly accessibilityHint?: string;
  readonly style: Record<string, string | number>;
}

export interface UniversalComponentContract<TState = Record<string, unknown>> {
  readonly component: string;
  readonly preset: YoloPresetId;
  readonly state: TState;
  readonly domProps: UniversalDomProps;
  readonly nativeProps: UniversalReactNativeProps;
}

function resolveTheme(themeOrPreset?: YoloTheme | YoloPresetId): YoloTheme {
  if (!themeOrPreset) return createYoloTheme("neutral", true);
  if (typeof themeOrPreset === "string")
    return createYoloTheme(themeOrPreset, true);
  return themeOrPreset;
}

// ============================================================================
// 1. Button (Migrated from @aphrody/m3-primitives/Button & @aphrody/m3-react/button)
// ============================================================================

export type YoloButtonVariant =
  "filled" | "tonal" | "outlined" | "text" | "elevated";
export type YoloButtonSize = "xs" | "sm" | "md" | "lg" | "icon";

export interface YoloButtonOptions {
  readonly variant?: YoloButtonVariant;
  readonly size?: YoloButtonSize;
  readonly disabled?: boolean;
  readonly loading?: boolean;
  readonly label?: string;
  readonly theme?: YoloTheme | YoloPresetId;
}

const BUTTON_HEIGHT_PX: Record<YoloButtonSize, number> = {
  xs: 28,
  sm: 32,
  md: 36,
  lg: 40,
  icon: 32,
};

const BUTTON_PX_PADDING: Record<YoloButtonSize, number> = {
  xs: 12,
  sm: 16,
  md: 20,
  lg: 24,
  icon: 0,
};

/* @__PURE__ */
export function Button(
  options: YoloButtonOptions = {},
): UniversalComponentContract<{
  variant: YoloButtonVariant;
  size: YoloButtonSize;
  disabled: boolean;
  loading: boolean;
}> {
  const theme = resolveTheme(options.theme);
  const variant = options.variant ?? "filled";
  const size = options.size ?? "md";
  const loading = Boolean(options.loading);
  const disabled = Boolean(options.disabled || loading);

  const height = BUTTON_HEIGHT_PX[size];
  const padX = BUTTON_PX_PADDING[size];
  const radius =
    theme.preset === "omarchy"
      ? theme.style.radiusSmPx
      : theme.preset === "fluent"
        ? theme.style.radiusMdPx
        : theme.style.radiusPillPx;

  const bg =
    variant === "filled"
      ? theme.tokens.accentPrimary
      : variant === "tonal"
        ? theme.tokens.bgElevated
        : variant === "elevated"
          ? theme.tokens.bgSurface
          : "transparent";

  const fg =
    variant === "filled"
      ? theme.tokens.bgPrimary
      : variant === "outlined" || variant === "text"
        ? theme.tokens.accentPrimary
        : theme.tokens.textPrimary;

  const borderColor =
    variant === "outlined" ? theme.tokens.borderSubtle : "transparent";

  const sharedStyle: Record<string, string | number> = {
    height,
    paddingLeft: padX,
    paddingRight: padX,
    borderRadius: radius,
    backgroundColor: bg,
    color: fg,
    borderWidth: variant === "outlined" ? theme.style.borderWidthPx : 0,
    borderColor,
    opacity: disabled ? 0.38 : 1,
  };

  return {
    component: "Button",
    preset: theme.preset,
    state: { variant, size, disabled, loading },
    domProps: {
      role: "button",
      className: `yolo-btn yolo-btn-${variant} yolo-btn-${size} inline-flex items-center justify-center select-none`,
      style: sharedStyle,
      "data-yolo-component": "Button",
      "data-yolo-preset": theme.preset,
      "aria-disabled": disabled || undefined,
      "aria-busy": loading || undefined,
      "aria-label": options.label,
      tabIndex: disabled ? -1 : 0,
    },
    nativeProps: {
      accessibilityRole: "button",
      accessibilityState: { disabled, busy: loading },
      accessibilityLabel: options.label,
      style: {
        ...sharedStyle,
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "center",
      },
    },
  };
}

// ============================================================================
// 2. Card (Migrated from @aphrody/m3-primitives/Card)
// ============================================================================

export type YoloCardVariant = "elevated" | "filled" | "outlined";

export interface YoloCardOptions {
  readonly variant?: YoloCardVariant;
  readonly interactive?: boolean;
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Card(
  options: YoloCardOptions = {},
): UniversalComponentContract<{
  variant: YoloCardVariant;
  interactive: boolean;
}> {
  const theme = resolveTheme(options.theme);
  const variant = options.variant ?? "elevated";
  const interactive = Boolean(options.interactive);
  const bg =
    variant === "elevated"
      ? theme.tokens.bgElevated
      : variant === "filled"
        ? theme.tokens.bgSurface
        : theme.tokens.bgPrimary;

  const style: Record<string, string | number> = {
    backgroundColor: bg,
    borderRadius: theme.style.radiusLgPx,
    borderWidth: theme.style.borderWidthPx,
    borderColor: theme.tokens.borderSubtle,
    padding: theme.style.spacingGridPx * 4,
  };

  return {
    component: "Card",
    preset: theme.preset,
    state: { variant, interactive },
    domProps: {
      role: interactive ? "button" : "region",
      className: `yolo-card yolo-card-${variant} flex flex-col`,
      style,
      "data-yolo-component": "Card",
      "data-yolo-preset": theme.preset,
      tabIndex: interactive ? 0 : undefined,
    },
    nativeProps: {
      accessibilityRole: interactive ? "button" : "none",
      accessibilityState: {},
      style: {
        ...style,
        flexDirection: "column",
      },
    },
  };
}

// ============================================================================
// 3. Dialog (Migrated from @aphrody/m3-primitives/Dialog)
// ============================================================================

export interface YoloDialogOptions {
  readonly open?: boolean;
  readonly modal?: boolean;
  readonly title?: string;
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Dialog(
  options: YoloDialogOptions = {},
): UniversalComponentContract<{ open: boolean; modal: boolean }> {
  const theme = resolveTheme(options.theme);
  const open = options.open ?? false;
  const modal = options.modal ?? true;

  const style: Record<string, string | number> = {
    backgroundColor: theme.tokens.bgElevated,
    borderRadius: theme.style.radiusLgPx,
    borderWidth: theme.style.borderWidthPx,
    borderColor: theme.tokens.borderSubtle,
    padding: theme.style.spacingGridPx * 6,
  };

  return {
    component: "Dialog",
    preset: theme.preset,
    state: { open, modal },
    domProps: {
      role: "dialog",
      className: `yolo-dialog ${open ? "yolo-dialog-open" : "yolo-dialog-closed"}`,
      style,
      "data-yolo-component": "Dialog",
      "data-yolo-preset": theme.preset,
      "aria-expanded": open,
      "aria-label": options.title,
    },
    nativeProps: {
      accessibilityRole: "alert",
      accessibilityState: { expanded: open },
      accessibilityLabel: options.title,
      style,
    },
  };
}

// ============================================================================
// 4. Input (Migrated from @aphrody/m3-primitives/Input)
// ============================================================================

export interface YoloInputOptions {
  readonly value?: string;
  readonly placeholder?: string;
  readonly disabled?: boolean;
  readonly invalid?: boolean;
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Input(
  options: YoloInputOptions = {},
): UniversalComponentContract<{
  value: string;
  disabled: boolean;
  invalid: boolean;
}> {
  const theme = resolveTheme(options.theme);
  const value = options.value ?? "";
  const disabled = Boolean(options.disabled);
  const invalid = Boolean(options.invalid);

  const style: Record<string, string | number> = {
    backgroundColor: theme.tokens.bgSurface,
    color: theme.tokens.textPrimary,
    borderRadius: theme.style.radiusMdPx,
    borderWidth: theme.style.borderWidthPx,
    borderColor: invalid ? "#ef4444" : theme.tokens.borderSubtle,
    paddingHorizontal: theme.style.spacingGridPx * 3,
    paddingVertical: theme.style.spacingGridPx * 2,
    height: 40,
  };

  return {
    component: "Input",
    preset: theme.preset,
    state: { value, disabled, invalid },
    domProps: {
      role: "textbox",
      className: "yolo-input w-full",
      style,
      "data-yolo-component": "Input",
      "data-yolo-preset": theme.preset,
      "aria-disabled": disabled || undefined,
      "aria-invalid": invalid || undefined,
      "aria-label": options.placeholder,
    },
    nativeProps: {
      accessibilityRole: "text",
      accessibilityState: { disabled },
      accessibilityLabel: options.placeholder,
      style,
    },
  };
}

// ============================================================================
// 5. Select (Migrated from @aphrody/m3-primitives/Select)
// ============================================================================

export interface YoloSelectOptions {
  readonly value?: string;
  readonly open?: boolean;
  readonly disabled?: boolean;
  readonly options?: readonly string[];
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Select(
  opts: YoloSelectOptions = {},
): UniversalComponentContract<{
  value: string;
  open: boolean;
  disabled: boolean;
  options: readonly string[];
}> {
  const theme = resolveTheme(opts.theme);
  const items = opts.options ?? [];
  const value = opts.value ?? items[0] ?? "";
  const open = Boolean(opts.open);
  const disabled = Boolean(opts.disabled);

  const style: Record<string, string | number> = {
    backgroundColor: theme.tokens.bgSurface,
    color: theme.tokens.textPrimary,
    borderRadius: theme.style.radiusMdPx,
    borderWidth: theme.style.borderWidthPx,
    borderColor: theme.tokens.borderSubtle,
    height: 40,
    paddingHorizontal: theme.style.spacingGridPx * 3,
  };

  return {
    component: "Select",
    preset: theme.preset,
    state: { value, open, disabled, options: items },
    domProps: {
      role: "combobox",
      className: "yolo-select inline-flex items-center justify-between",
      style,
      "data-yolo-component": "Select",
      "data-yolo-preset": theme.preset,
      "aria-expanded": open,
      "aria-disabled": disabled || undefined,
    },
    nativeProps: {
      accessibilityRole: "combobox",
      accessibilityState: { expanded: open, disabled },
      accessibilityLabel: value,
      style,
    },
  };
}

// ============================================================================
// 6. Switch (Migrated from @aphrody/m3-primitives/Switch)
// ============================================================================

export interface YoloSwitchOptions {
  readonly checked?: boolean;
  readonly disabled?: boolean;
  readonly label?: string;
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Switch(
  options: YoloSwitchOptions = {},
): UniversalComponentContract<{ checked: boolean; disabled: boolean }> {
  const theme = resolveTheme(options.theme);
  const checked = Boolean(options.checked);
  const disabled = Boolean(options.disabled);

  const style: Record<string, string | number> = {
    width: 52,
    height: 32,
    borderRadius: theme.style.radiusPillPx,
    backgroundColor: checked
      ? theme.tokens.accentPrimary
      : theme.tokens.bgElevated,
    borderWidth: theme.style.borderWidthPx,
    borderColor: checked
      ? theme.tokens.accentPrimary
      : theme.tokens.borderSubtle,
    opacity: disabled ? 0.38 : 1,
  };

  return {
    component: "Switch",
    preset: theme.preset,
    state: { checked, disabled },
    domProps: {
      role: "switch",
      className: "yolo-switch inline-flex items-center cursor-pointer",
      style,
      "data-yolo-component": "Switch",
      "data-yolo-preset": theme.preset,
      "aria-checked": checked,
      "aria-disabled": disabled || undefined,
      "aria-label": options.label,
      tabIndex: disabled ? -1 : 0,
    },
    nativeProps: {
      accessibilityRole: "switch",
      accessibilityState: { checked, disabled },
      accessibilityLabel: options.label,
      style,
    },
  };
}

// ============================================================================
// 7. Tabs (Migrated from @aphrody/m3-primitives/Tabs)
// ============================================================================

export interface YoloTabsOptions {
  readonly tabs: readonly string[];
  readonly activeTab?: string;
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Tabs(options: YoloTabsOptions): UniversalComponentContract<{
  tabs: readonly string[];
  activeTab: string;
  activeIndex: number;
}> {
  const theme = resolveTheme(options.theme);
  const activeTab = options.activeTab ?? options.tabs[0] ?? "";
  const activeIndex = Math.max(0, options.tabs.indexOf(activeTab));

  const style: Record<string, string | number> = {
    backgroundColor: theme.tokens.bgSurface,
    borderRadius: theme.style.radiusMdPx,
    padding: theme.style.spacingGridPx,
    gap: theme.style.spacingGridPx,
  };

  return {
    component: "Tabs",
    preset: theme.preset,
    state: { tabs: options.tabs, activeTab, activeIndex },
    domProps: {
      role: "tablist",
      className: "yolo-tabs inline-flex items-center",
      style,
      "data-yolo-component": "Tabs",
      "data-yolo-preset": theme.preset,
      "aria-selected": true,
    },
    nativeProps: {
      accessibilityRole: "tablist",
      accessibilityState: { selected: true },
      accessibilityLabel: activeTab,
      style: {
        ...style,
        flexDirection: "row",
      },
    },
  };
}

// ============================================================================
// 8. Toast (Migrated from @aphrody/m3-primitives/Toast)
// ============================================================================

export interface YoloToastOptions {
  readonly message: string;
  readonly tone?: "info" | "success" | "warning" | "error";
  readonly durationMs?: number;
  readonly theme?: YoloTheme | YoloPresetId;
}

/* @__PURE__ */
export function Toast(options: YoloToastOptions): UniversalComponentContract<{
  message: string;
  tone: string;
  durationMs: number;
}> {
  const theme = resolveTheme(options.theme);
  const tone = options.tone ?? "info";
  const durationMs = options.durationMs ?? 4000;

  const style: Record<string, string | number> = {
    backgroundColor: theme.tokens.bgElevated,
    color: theme.tokens.textPrimary,
    borderRadius: theme.style.radiusMdPx,
    borderWidth: theme.style.borderWidthPx,
    borderColor: theme.tokens.accentPrimary,
    paddingHorizontal: theme.style.spacingGridPx * 4,
    paddingVertical: theme.style.spacingGridPx * 3,
  };

  return {
    component: "Toast",
    preset: theme.preset,
    state: { message: options.message, tone, durationMs },
    domProps: {
      role: "status",
      className: `yolo-toast yolo-toast-${tone}`,
      style,
      "data-yolo-component": "Toast",
      "data-yolo-preset": theme.preset,
      "aria-label": options.message,
    },
    nativeProps: {
      accessibilityRole: "alert",
      accessibilityState: {},
      accessibilityLabel: options.message,
      style,
    },
  };
}

// ============================================================================
// 9. FormField (Migrated from @aphrody/m3-forms)
// ============================================================================

export interface YoloFormFieldOptions {
  readonly name: string;
  readonly label: string;
  readonly value?: string;
  readonly required?: boolean;
  readonly error?: string;
  readonly description?: string;
  readonly theme?: YoloTheme | YoloPresetId;
}

export type YoloFormFieldContract = UniversalComponentContract<{
  name: string;
  value: string;
  invalid: boolean;
  errorMessage?: string;
  descriptionId: string;
  errorId: string;
}>;

/* @__PURE__ */
export function FormField(
  options: YoloFormFieldOptions,
): YoloFormFieldContract {
  const theme = resolveTheme(options.theme);
  const value = options.value ?? "";
  const invalid =
    Boolean(options.error) ||
    (Boolean(options.required) && value.trim().length === 0);
  const errorMessage =
    options.error ?? (invalid ? `${options.label} is required` : undefined);
  const descriptionId = `yolo-field-${options.name}-desc`;
  const errorId = `yolo-field-${options.name}-err`;

  const style: Record<string, string | number> = {
    gap: theme.style.spacingGridPx * 1.5,
  };

  return {
    component: "FormField",
    preset: theme.preset,
    state: {
      name: options.name,
      value,
      invalid,
      errorMessage,
      descriptionId,
      errorId,
    },
    domProps: {
      role: "group",
      className: "yolo-form-field flex flex-col",
      style,
      "data-yolo-component": "FormField",
      "data-yolo-preset": theme.preset,
      "aria-invalid": invalid || undefined,
      "aria-describedby": invalid
        ? `${descriptionId} ${errorId}`
        : descriptionId,
      "aria-label": options.label,
    },
    nativeProps: {
      accessibilityRole: "none",
      accessibilityState: {},
      accessibilityLabel: options.label,
      accessibilityHint: errorMessage ?? options.description,
      style: {
        ...style,
        flexDirection: "column",
      },
    },
  };
}

// ============================================================================
// 10. SpringMotion (Migrated from @aphrody/m3-motion/springs)
// ============================================================================

export interface SpringPhysicsSpec {
  readonly type: "spring";
  readonly stiffness: number;
  readonly damping: number;
  readonly mass: number;
}

export const yoloSprings = {
  fast: {
    spatial: {
      type: "spring",
      stiffness: 400,
      damping: 24,
      mass: 0.8,
    } as const,
    effects: {
      type: "spring",
      stiffness: 350,
      damping: 37,
      mass: 0.8,
    } as const,
  },
  default: {
    spatial: {
      type: "spring",
      stiffness: 220,
      damping: 17,
      mass: 1.0,
    } as const,
    effects: {
      type: "spring",
      stiffness: 180,
      damping: 26,
      mass: 1.0,
    } as const,
  },
  slow: {
    spatial: { type: "spring", stiffness: 80, damping: 12, mass: 1.2 } as const,
    effects: { type: "spring", stiffness: 70, damping: 16, mass: 1.2 } as const,
  },
} as const;

/* @__PURE__ */
export function SpringMotion(
  speed: "fast" | "default" | "slow" = "default",
  category: "spatial" | "effects" = "spatial",
  tSeconds = 0.16,
): {
  readonly spec: SpringPhysicsSpec;
  readonly progressAtT: number;
  readonly cssTransition: string;
  readonly reanimatedConfig: {
    stiffness: number;
    damping: number;
    mass: number;
  };
} {
  const spec = yoloSprings[speed][category];
  const omega0 = Math.sqrt(spec.stiffness / spec.mass);
  const zeta = spec.damping / (2 * Math.sqrt(spec.stiffness * spec.mass));
  const envelope = Math.exp(-zeta * omega0 * tSeconds);
  const progressAtT =
    zeta < 1
      ? 1 - envelope * Math.cos(omega0 * Math.sqrt(1 - zeta * zeta) * tSeconds)
      : 1 - envelope * (1 + omega0 * tSeconds);

  return {
    spec,
    progressAtT: Number(progressAtT.toFixed(4)),
    cssTransition: `all ${speed === "fast" ? "180ms" : speed === "default" ? "300ms" : "500ms"} cubic-bezier(0.2, 0.0, 0, 1.0)`,
    reanimatedConfig: {
      stiffness: spec.stiffness,
      damping: spec.damping,
      mass: spec.mass,
    },
  };
}
