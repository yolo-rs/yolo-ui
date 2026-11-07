declare module "node:path" {
  export function join(...paths: string[]): string;
  export function resolve(...paths: string[]): string;
  export function dirname(path: string): string;
  export function basename(path: string, ext?: string): string;
  export function relative(from: string, to: string): string;
}

declare module "node:fs" {
  export interface Dirent {
    name: string;
    isDirectory(): boolean;
    isFile(): boolean;
  }
  export function existsSync(path: string): boolean;
  export function readFileSync(
    path: string,
    encoding: "utf8" | "utf-8",
  ): string;
  export function readdirSync(
    path: string,
    options: { withFileTypes: true },
  ): Dirent[];
  export function readdirSync(path: string): string[];
  export function renameSync(oldPath: string, newPath: string): void;
  export function statSync(path: string): { size: number; mtimeMs: number; isDirectory(): boolean };
}

declare module "node:fs/promises" {
  export function link(existingPath: string, newPath: string): Promise<void>;
  export function unlink(path: string): Promise<void>;
  export function mkdir(
    path: string,
    options?: { recursive?: boolean },
  ): Promise<string | undefined>;
  export function rename(oldPath: string, newPath: string): Promise<void>;
  export function rm(
    path: string,
    options?: { force?: boolean; recursive?: boolean },
  ): Promise<void>;
  export function stat(
    path: string,
  ): Promise<{ size: number; mtimeMs: number; isDirectory(): boolean }>;
  export function readdir(path: string): Promise<string[]>;
  export function mkdtemp(prefix: string): Promise<string>;
}

declare module "node:os" {
  export function tmpdir(): string;
}

interface ReadableStream<R = any> {
  text(): Promise<string>;
}

interface BunShellPromise extends Promise<{
  stdout: Uint8Array;
  stderr: Uint8Array;
  exitCode: number;
  text(): string;
}> {
  text(): Promise<string>;
  json(): Promise<unknown>;
  quiet(): BunShellPromise;
  nothrow(): BunShellPromise;
  cwd(dir: string): BunShellPromise;
}

declare module "bun" {
  export interface BunPluginBuilder {
    onResolve(
      options: { filter: RegExp; namespace?: string },
      callback: (args: {
        path: string;
        importer: string;
        namespace: string;
      }) => { path: string; namespace?: string } | undefined,
    ): void;
    onLoad(
      options: { filter: RegExp; namespace?: string },
      callback: (args: { path: string; namespace: string }) =>
        | {
            loader: "js" | "ts" | "tsx" | "jsx" | "css" | "json";
            contents: string;
          }
        | Promise<{
            loader: "js" | "ts" | "tsx" | "jsx" | "css" | "json";
            contents: string;
          }>,
    ): void;
  }

  export interface BunPlugin {
    name: string;
    setup(build: BunPluginBuilder): void | Promise<void>;
  }

  export function $(
    strings: TemplateStringsArray,
    ...expressions: unknown[]
  ): BunShellPromise;
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
    toHaveLength(expected: number): void;
    toBeDefined(): void;
    toBeGreaterThan(expected: number): void;
    toBeGreaterThanOrEqual(expected: number): void;
  };
}

declare namespace Bun {
  const argv: string[];
  const version: string;
  const env: Record<string, string | undefined>;
  const stdin: {
    stream(): AsyncIterable<Uint8Array>;
    text(): Promise<string>;
  };
  function file(path: string): {
    exists(): Promise<boolean>;
    text(): Promise<string>;
    json(): Promise<unknown>;
    bytes(): Promise<Uint8Array>;
    stat(): Promise<{ size: number; mtimeMs: number }>;
    delete(): Promise<void>;
  };
  function write(
    dest: string,
    input: string | Uint8Array | Blob | Response,
  ): Promise<number>;
  function resolveSync(specifier: string, parent: string): string;
  function plugin(plg: import("bun").BunPlugin): void;
  function build(options: {
    entrypoints: string[];
    outdir?: string;
    target?: "browser" | "bun" | "node";
    minify?: boolean;
    sourcemap?: "none" | "linked" | "inline" | "external";
    splitting?: boolean;
    plugins?: import("bun").BunPlugin[];
    define?: Record<string, string>;
  }): Promise<{
    success: boolean;
    outputs: Array<{
      path: string;
      kind: string;
      size: number;
      text(): Promise<string>;
    }>;
    logs: Array<{ message: string; level?: string }>;
  }>;
  function serve(options: Record<string, unknown>): {
    port: number;
    hostname: string;
    url: URL;
    stop(closeActiveConnections?: boolean): void;
  };
  function spawn(
    cmd: string[],
    options?: {
      cwd?: string;
      env?: Record<string, string | undefined>;
      stdout?: "pipe" | "inherit" | "ignore";
      stderr?: "pipe" | "inherit" | "ignore";
    },
  ): {
    stdout: ReadableStream<Uint8Array>;
    stderr: ReadableStream<Uint8Array>;
    exited: Promise<number>;
  };
  function spawnSync(
    cmd: string[],
    options?: {
      cwd?: string;
      env?: Record<string, string | undefined>;
      stdout?: "pipe" | "inherit" | "ignore";
      stderr?: "pipe" | "inherit" | "ignore";
    },
  ): {
    stdout: Uint8Array;
    stderr: Uint8Array;
    exitCode: number;
    success: boolean;
  };
  function $(
    strings: TemplateStringsArray,
    ...expressions: unknown[]
  ): BunShellPromise;
  namespace TOML {
    function parse(input: string): unknown;
  }
  class Glob {
    constructor(pattern: string);
    scan(options?: {
      cwd?: string;
      onlyFiles?: boolean;
    }): AsyncIterable<string>;
    scanSync(options?: { cwd?: string; onlyFiles?: boolean }): Iterable<string>;
  }
  class Transpiler {
    constructor(options?: { loader?: "ts" | "tsx" | "js" | "jsx" });
    transformSync(code: string): string;
  }
  class CryptoHasher {
    constructor(algorithm: string);
    update(data: string | Uint8Array): this;
    digest(encoding: "hex" | "base64"): string;
    digest(): Uint8Array;
  }
}

interface ImportMeta {
  readonly dir: string;
  readonly file: string;
  readonly path: string;
  readonly main: boolean;
}

declare const process: {
  readonly argv: string[];
  readonly pid: number;
  readonly platform: string;
  readonly execPath: string;
  readonly env: Record<string, string | undefined>;
  cwd(): string;
  exit(code?: number): never;
  readonly stdout: {
    write(chunk: string | Uint8Array): boolean;
  };
  readonly stderr: {
    write(chunk: string | Uint8Array): boolean;
  };
};
