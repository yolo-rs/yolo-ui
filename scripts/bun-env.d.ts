declare module "node:path" {
  export function join(...paths: string[]): string;
  export function resolve(...paths: string[]): string;
}

declare module "bun:test" {
  export function describe(name: string, fn: () => void | Promise<void>): void;
  export function test(
    name: string,
    fn: () => void | Promise<void>,
    timeout?: number,
  ): void;
  export function expect(actual: unknown): {
    toBe(expected: unknown): void;
    toEqual(expected: unknown): void;
    toContain(expected: unknown): void;
    toBeGreaterThan(expected: number): void;
    toBeGreaterThanOrEqual(expected: number): void;
  };
}

declare namespace Bun {
  const argv: string[];
  function file(path: string): {
    exists(): Promise<boolean>;
    text(): Promise<string>;
    json(): Promise<unknown>;
  };
  function write(dest: string, input: string | Uint8Array): Promise<number>;
  function spawn(
    cmd: string[],
    options?: {
      cwd?: string;
      stdout?: "pipe" | "inherit" | "ignore";
      stderr?: "pipe" | "inherit" | "ignore";
    },
  ): {
    stdout: ReadableStream<Uint8Array>;
    exited: Promise<number>;
  };
  namespace TOML {
    function parse(input: string): unknown;
  }
  class Glob {
    constructor(pattern: string);
    scan(options?: {
      cwd?: string;
      onlyFiles?: boolean;
    }): AsyncIterable<string>;
  }
  class Transpiler {
    constructor(options?: { loader?: "ts" | "tsx" | "js" | "jsx" });
    transformSync(code: string): string;
  }
}

interface ImportMeta {
  readonly dir: string;
  readonly file: string;
  readonly path: string;
  readonly main: boolean;
}

declare const process: {
  exit(code?: number): never;
};
