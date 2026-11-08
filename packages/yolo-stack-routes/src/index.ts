// Shared route table used by every showcase app so the benchmark harness hits identical pages.
// The Rust twin lives in crates/yolo-stack-routes/src/lib.rs; keep both in sync.

export interface StackRoute {
  readonly id: string;
  readonly path: string;
  /** Number of list rows the page renders server-side. */
  readonly rows: number;
  readonly interactive: boolean;
}

export const ROUTES: readonly StackRoute[] = [
  { id: "home", path: "/", rows: 0, interactive: false },
  { id: "links-100", path: "/links", rows: 100, interactive: false },
  { id: "table-1k", path: "/table", rows: 1000, interactive: true },
  { id: "island-counter", path: "/island", rows: 0, interactive: true },
  { id: "api-json", path: "/api/health", rows: 0, interactive: false },
] as const;

export function route(id: string): StackRoute | undefined {
  return ROUTES.find((r) => r.id === id);
}
