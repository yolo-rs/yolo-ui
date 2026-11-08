import { expect, test } from "bun:test";
import { ROUTES, route } from "./index.ts";

test("ids are unique", () => {
  expect(new Set(ROUTES.map((r) => r.id)).size).toBe(ROUTES.length);
});

test("lookup", () => {
  expect(route("table-1k")?.rows).toBe(1000);
  expect(route("missing")).toBeUndefined();
});
