// SPDX-License-Identifier: MIT OR Apache-2.0
// The `/_next/image` optimizer: validates the request the way Next.js does (allowed widths, qualities, remote and
// local patterns, SVG policy), fetches the source (a build file or a remote URL), negotiates AVIF/WebP from `Accept`,
// resizes through sharp when installed (pass-through otherwise) and caches the result in memory and on disk.
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import type {
  LocalPattern,
  RemotePattern,
  ResolvedImages,
} from "../config/next-config";

export const AVIF = "image/avif";
export const WEBP = "image/webp";
export const PNG = "image/png";
export const JPEG = "image/jpeg";
export const GIF = "image/gif";
export const SVG = "image/svg+xml";
export const ICO = "image/x-icon";
export const BMP = "image/bmp";
export const TIFF = "image/tiff";

export type ResizeFormat =
  typeof AVIF | typeof WEBP | typeof JPEG | typeof PNG | undefined;

export interface ResizeOptions {
  width: number;
  quality: number;
  /** The negotiated output format, or `undefined` when the source format is unknown. */
  format: ResizeFormat;
}

export interface ResizeResult {
  bytes: Uint8Array;
  contentType: string;
  /** The bytes are the untouched source (reported through `x-yolo-image: passthrough`). */
  passthrough?: boolean;
}

export type Resizer = (
  bytes: Uint8Array,
  opts: ResizeOptions,
) => Promise<ResizeResult>;

export interface ImageOptimizerOptions {
  images: ResolvedImages;
  /** A build or `public/` file by URL pathname (`/logo.png`). */
  local: (pathname: string) => Promise<Blob | undefined>;
  /** Disk cache directory; memory only when omitted. */
  cacheDir?: string;
  /** Defaults to sharp when installed, else pass-through. */
  resize?: Resizer;
  fetch?: typeof fetch;
  now?: () => number;
  /** Memory cache bounds. */
  memoryEntries?: number;
  memoryBytes?: number;
  /** Remote fetch timeout in milliseconds. */
  timeout?: number;
}

export interface ImageOptimizer {
  handle(request: Request): Promise<Response>;
  matches(pathname: string): boolean;
}

/** Second path the optimizer answers on, next to `images.path`. */
export const YOLO_IMAGE_PATH = "/_yolo/image";
const MAX_URL_LENGTH = 3072;
const DEFAULT_TIMEOUT = 7000;

class ImageError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

// Content type from magic bytes (Next.js `detectContentType`).
const ascii = (bytes: Uint8Array, start: number, end: number) =>
  String.fromCharCode(...bytes.subarray(start, end));

/** The MIME type of an encoded image, or `undefined`. */
export function detectContentType(bytes: Uint8Array): string | undefined {
  if (bytes.length < 12)
    return bytes.length >= 2 && bytes[0] === 0x3c ? maybeSvg(bytes) : undefined;
  if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return JPEG;
  if (
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  )
    return PNG;
  if (bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) return GIF;
  if (ascii(bytes, 0, 4) === "RIFF" && ascii(bytes, 8, 12) === "WEBP")
    return WEBP;
  if (
    ascii(bytes, 4, 8) === "ftyp" &&
    ["avif", "avis"].includes(ascii(bytes, 8, 12))
  )
    return AVIF;
  if (
    bytes[0] === 0x00 &&
    bytes[1] === 0x00 &&
    bytes[2] === 0x01 &&
    bytes[3] === 0x00
  )
    return ICO;
  if (bytes[0] === 0x42 && bytes[1] === 0x4d) return BMP;
  if (
    (bytes[0] === 0x49 && bytes[1] === 0x49 && bytes[2] === 0x2a) ||
    (bytes[0] === 0x4d && bytes[1] === 0x4d && bytes[2] === 0x00)
  )
    return TIFF;
  if (
    bytes[0] === 0x3c ||
    (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf)
  )
    return maybeSvg(bytes);
  return undefined;
}

function maybeSvg(bytes: Uint8Array): string | undefined {
  const head = new TextDecoder().decode(bytes.subarray(0, 4096));
  return /<svg[\s>]/i.test(head) || /<\?xml/i.test(head) ? SVG : undefined;
}

/** Whether the GIF, APNG or WebP has several frames. */
export function isAnimated(
  bytes: Uint8Array,
  contentType: string | undefined,
): boolean {
  if (contentType === GIF) {
    let descriptors = 0;
    for (let i = 13; i < bytes.length - 4; i++) {
      // A NETSCAPE2.0 application extension carries the loop count of an animation.
      if (
        bytes[i] === 0x21 &&
        bytes[i + 1] === 0xff &&
        bytes[i + 2] === 0x0b &&
        ascii(bytes, i + 3, i + 14) === "NETSCAPE2.0"
      )
        return true;
      // A graphic control extension precedes each frame.
      if (
        bytes[i] === 0x21 &&
        bytes[i + 1] === 0xf9 &&
        bytes[i + 2] === 0x04 &&
        ++descriptors > 1
      )
        return true;
    }
    return false;
  }
  if (contentType === PNG) {
    for (let i = 8; i + 8 <= bytes.length;) {
      const length =
        (bytes[i]! << 24) |
        (bytes[i + 1]! << 16) |
        (bytes[i + 2]! << 8) |
        bytes[i + 3]!;
      const type = ascii(bytes, i + 4, i + 8);
      if (type === "acTL") return true;
      if (type === "IDAT" || type === "IEND") return false;
      i += 12 + length;
    }
    return false;
  }
  if (contentType === WEBP) {
    if (ascii(bytes, 12, 16) !== "VP8X") return false;
    return (bytes[20]! & 0x02) !== 0;
  }
  return false;
}

// Pattern matching (Next.js `match-remote-pattern` / `match-local-pattern`): `*` is one segment, `**` any number.
function globToRegExp(glob: string, separator: string): RegExp {
  const sep = separator.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  let out = "";
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i]!;
    if (c === "*") {
      if (glob[i + 1] === "*") {
        out += ".*";
        i++;
      } else out += `[^${sep}]*`;
    } else out += c.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp(`^${out}$`);
}

export function matchRemotePattern(pattern: RemotePattern, url: URL): boolean {
  if (
    pattern.protocol !== undefined &&
    pattern.protocol.replace(/:$/, "") !== url.protocol.replace(/:$/, "")
  )
    return false;
  if (pattern.port !== undefined && pattern.port !== url.port) return false;
  if (pattern.hostname === undefined)
    throw new Error(
      "@yolo-rs/fullstack: images.remotePatterns[].hostname is required",
    );
  if (!globToRegExp(pattern.hostname, ".").test(url.hostname)) return false;
  if (pattern.search !== undefined && pattern.search !== url.search)
    return false;
  if (
    pattern.pathname !== undefined &&
    !globToRegExp(pattern.pathname, "/").test(url.pathname)
  )
    return false;
  return true;
}

export function matchLocalPattern(pattern: LocalPattern, url: URL): boolean {
  if (pattern.search !== undefined && pattern.search !== url.search)
    return false;
  if (
    pattern.pathname !== undefined &&
    !globToRegExp(pattern.pathname, "/").test(url.pathname)
  )
    return false;
  return true;
}

export interface ValidatedParams {
  href: string;
  width: number;
  quality: number;
  isAbsolute: boolean;
  /** Negotiated output type, or `undefined` to keep the source type. */
  mimeType: typeof AVIF | typeof WEBP | undefined;
}

/** Validate `url`, `w` and `q` against the images config; throws `ImageError` (400) when invalid. */
export function validateParams(
  images: ResolvedImages,
  query: URLSearchParams,
  accept: string,
): ValidatedParams {
  const url = query.get("url");
  const w = query.get("w");
  const q = query.get("q");
  if (!url) throw new ImageError(400, '"url" parameter is required');
  if (query.getAll("url").length > 1)
    throw new ImageError(400, '"url" parameter cannot be an array');
  if (url.length > MAX_URL_LENGTH)
    throw new ImageError(400, '"url" parameter is too long');
  if (url.startsWith("//"))
    throw new ImageError(
      400,
      '"url" parameter cannot be a protocol-relative URL (//)',
    );

  let href: string;
  let isAbsolute: boolean;
  if (url.startsWith("/")) {
    href = url;
    isAbsolute = false;
    const parsed = new URL(url, "http://n");
    let pathname: string;
    try {
      pathname = decodeURIComponent(parsed.pathname);
    } catch {
      throw new ImageError(400, '"url" parameter is invalid');
    }
    if (
      /\/(_next|_yolo)\/image($|\/)/.test(pathname) ||
      pathname === images.path
    )
      throw new ImageError(400, '"url" parameter cannot be recursive');
    if (
      images.localPatterns &&
      !images.localPatterns.some((p) => matchLocalPattern(p, parsed))
    )
      throw new ImageError(400, '"url" parameter is not allowed');
  } else {
    let parsed: URL;
    try {
      parsed = new URL(url);
    } catch {
      throw new ImageError(400, '"url" parameter is invalid');
    }
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:")
      throw new ImageError(400, '"url" parameter is invalid');
    if (!images.remotePatterns.some((p) => matchRemotePattern(p, parsed)))
      throw new ImageError(400, '"url" parameter is not allowed');
    href = parsed.toString();
    isAbsolute = true;
  }

  if (!w) throw new ImageError(400, '"w" parameter (width) is required');
  if (query.getAll("w").length > 1)
    throw new ImageError(400, '"w" parameter (width) cannot be an array');
  if (!/^[0-9]+$/.test(w))
    throw new ImageError(
      400,
      '"w" parameter (width) must be an integer greater than 0',
    );
  const width = parseInt(w, 10);
  if (width <= 0 || Number.isNaN(width))
    throw new ImageError(
      400,
      '"w" parameter (width) must be an integer greater than 0',
    );
  const sizes = [...images.deviceSizes, ...images.imageSizes];
  if (!sizes.includes(width))
    throw new ImageError(
      400,
      `"w" parameter (width) of ${width} is not allowed`,
    );

  if (!q) throw new ImageError(400, '"q" parameter (quality) is required');
  if (query.getAll("q").length > 1)
    throw new ImageError(400, '"q" parameter (quality) cannot be an array');
  if (!/^[0-9]+$/.test(q))
    throw new ImageError(
      400,
      '"q" parameter (quality) must be an integer between 1 and 100',
    );
  const quality = parseInt(q, 10);
  if (quality < 1 || quality > 100)
    throw new ImageError(
      400,
      '"q" parameter (quality) must be an integer between 1 and 100',
    );
  if (!images.qualities.includes(quality))
    throw new ImageError(
      400,
      `"q" parameter (quality) of ${quality} is not allowed`,
    );

  return {
    href,
    width,
    quality,
    isAbsolute,
    mimeType: negotiate(images.formats, accept),
  };
}

/** The first configured format the client accepts. */
export function negotiate(
  formats: readonly string[],
  accept: string,
): typeof AVIF | typeof WEBP | undefined {
  const accepted = accept
    .split(",")
    .map((part) => part.split(";")[0]!.trim().toLowerCase())
    .filter(Boolean);
  for (const format of formats)
    if ((format === AVIF || format === WEBP) && accepted.includes(format))
      return format;
  return undefined;
}

/** `max-age` (or `s-maxage`) of a Cache-Control header, in seconds. */
export function getMaxAge(cacheControl: string | null): number {
  if (!cacheControl) return 0;
  const sMaxAge = /(?:^|,)\s*s-maxage=(\d+)/i.exec(cacheControl)?.[1];
  const maxAge = /(?:^|,)\s*max-age=(\d+)/i.exec(cacheControl)?.[1];
  const value = parseInt(sMaxAge ?? maxAge ?? "0", 10);
  return Number.isFinite(value) ? value : 0;
}

const EXTENSIONS: Record<string, string> = {
  [AVIF]: "avif",
  [WEBP]: "webp",
  [PNG]: "png",
  [JPEG]: "jpeg",
  [GIF]: "gif",
  [SVG]: "svg",
  [ICO]: "ico",
  [BMP]: "bmp",
  [TIFF]: "tiff",
};

/** `<basename of the source>.<extension of the output type>` for Content-Disposition. */
export function fileName(href: string, contentType: string): string {
  let pathname = href;
  try {
    pathname = new URL(href, "http://n").pathname;
  } catch {
    // keep the raw value
  }
  const base =
    (pathname.split("/").pop() || "image").replace(/\.[a-z0-9]+$/i, "") ||
    "image";
  const ext = EXTENSIONS[contentType];
  return ext ? `${base}.${ext}` : base;
}

// Resizers.

/** Keeps the source bytes; sharp is not consulted. */
export const passthroughResizer: Resizer = async (bytes) => ({
  bytes,
  contentType: detectContentType(bytes) ?? "application/octet-stream",
  passthrough: true,
});

type SharpModule = (
  input: Uint8Array,
  options?: Record<string, unknown>,
) => SharpInstance;
interface SharpInstance {
  rotate(): SharpInstance;
  resize(options: {
    width: number;
    withoutEnlargement: boolean;
  }): SharpInstance;
  avif(options: Record<string, unknown>): SharpInstance;
  webp(options: Record<string, unknown>): SharpInstance;
  png(options: Record<string, unknown>): SharpInstance;
  jpeg(options: Record<string, unknown>): SharpInstance;
  toBuffer(): Promise<Uint8Array>;
}

let sharpPromise: Promise<SharpModule | undefined> | undefined;
function loadSharp(): Promise<SharpModule | undefined> {
  // The specifier is a variable so bundlers do not try to resolve the optional dependency.
  const specifier = "sharp";
  sharpPromise ??= import(specifier)
    .then((mod) => (mod.default ?? mod) as SharpModule)
    .catch(() => undefined);
  return sharpPromise;
}

/** sharp when installed, pass-through otherwise. */
export const defaultResizer: Resizer = async (bytes, opts) => {
  const sharp = await loadSharp();
  if (!sharp) return passthroughResizer(bytes, opts);
  const source = detectContentType(bytes);
  const format =
    opts.format ??
    (source === PNG || source === JPEG || source === WEBP || source === AVIF
      ? source
      : JPEG);
  let image = sharp(bytes)
    .rotate()
    .resize({ width: opts.width, withoutEnlargement: true });
  switch (format) {
    case AVIF:
      image = image.avif({
        quality: Math.max(opts.quality - 20, 1),
        effort: 3,
      });
      break;
    case WEBP:
      image = image.webp({ quality: opts.quality });
      break;
    case PNG:
      image = image.png({ quality: opts.quality });
      break;
    default:
      image = image.jpeg({ quality: opts.quality, mozjpeg: true });
  }
  return { bytes: new Uint8Array(await image.toBuffer()), contentType: format };
};

// Cache.

interface CacheEntry {
  bytes: Uint8Array;
  contentType: string;
  /** Seconds the entry may be served for (the Cache-Control max-age). */
  maxAge: number;
  /** Absolute expiry in milliseconds. */
  expires: number;
  etag: string;
  passthrough: boolean;
}

interface CacheMeta extends Omit<CacheEntry, "bytes"> {
  version: 1;
}

class MemoryCache {
  private readonly map = new Map<string, CacheEntry>();
  private bytes = 0;
  constructor(
    private readonly maxEntries: number,
    private readonly maxBytes: number,
  ) {}
  get(key: string, now: number): CacheEntry | undefined {
    const entry = this.map.get(key);
    if (!entry) return undefined;
    if (entry.expires <= now) {
      this.delete(key);
      return undefined;
    }
    this.map.delete(key);
    this.map.set(key, entry);
    return entry;
  }
  set(key: string, entry: CacheEntry): void {
    this.delete(key);
    if (entry.bytes.byteLength > this.maxBytes) return;
    this.map.set(key, entry);
    this.bytes += entry.bytes.byteLength;
    for (const [oldest] of this.map) {
      if (this.map.size <= this.maxEntries && this.bytes <= this.maxBytes)
        break;
      this.delete(oldest);
    }
  }
  private delete(key: string): void {
    const entry = this.map.get(key);
    if (!entry) return;
    this.map.delete(key);
    this.bytes -= entry.bytes.byteLength;
  }
}

class DiskCache {
  private ready: Promise<void> | undefined;
  constructor(private readonly dir: string) {}
  private prepare(): Promise<void> {
    this.ready ??= mkdir(this.dir, { recursive: true }).then(() => undefined);
    return this.ready;
  }
  async get(key: string, now: number): Promise<CacheEntry | undefined> {
    const meta = Bun.file(join(this.dir, `${key}.json`));
    const data = Bun.file(join(this.dir, `${key}.bin`));
    try {
      if (!(await meta.exists()) || !(await data.exists())) return undefined;
      const parsed = (await meta.json()) as CacheMeta;
      if (parsed.version !== 1 || parsed.expires <= now) return undefined;
      return { ...parsed, bytes: new Uint8Array(await data.arrayBuffer()) };
    } catch {
      return undefined;
    }
  }
  async set(key: string, entry: CacheEntry): Promise<void> {
    try {
      await this.prepare();
      const { bytes, ...rest } = entry;
      const meta: CacheMeta = { ...rest, version: 1 };
      await Bun.write(join(this.dir, `${key}.bin`), bytes);
      await Bun.write(join(this.dir, `${key}.json`), JSON.stringify(meta));
    } catch (error) {
      console.warn(
        `@yolo-rs/fullstack: image cache write failed: ${String(error)}`,
      );
    }
  }
}

const hash = (input: string | Uint8Array) =>
  new Bun.CryptoHasher("sha256").update(input).digest("hex");

/** Create the `/_next/image` handler. */
export function createImageOptimizer(
  options: ImageOptimizerOptions,
): ImageOptimizer {
  const { images } = options;
  const now = options.now ?? (() => Date.now());
  const doFetch = options.fetch ?? fetch;
  const resize = options.resize ?? defaultResizer;
  const timeout = options.timeout ?? DEFAULT_TIMEOUT;
  const memory = new MemoryCache(
    options.memoryEntries ?? 256,
    options.memoryBytes ?? 64 * 1024 * 1024,
  );
  const disk = options.cacheDir ? new DiskCache(options.cacheDir) : undefined;
  // Collapses concurrent misses of one key into one optimization.
  const inflight = new Map<string, Promise<CacheEntry>>();

  const matches = (pathname: string) =>
    pathname === images.path || pathname === YOLO_IMAGE_PATH;

  const fetchSource = async (
    params: ValidatedParams,
  ): Promise<{ bytes: Uint8Array; contentType?: string; maxAge: number }> => {
    if (!params.isAbsolute) {
      const pathname = decodeURIComponent(
        new URL(params.href, "http://n").pathname,
      );
      const blob = await options.local(pathname);
      if (!blob) throw new ImageError(404, "image not found");
      return {
        bytes: new Uint8Array(await blob.arrayBuffer()),
        contentType: blob.type || undefined,
        maxAge: 0,
      };
    }
    let response: Response;
    try {
      response = await doFetch(params.href, {
        signal: AbortSignal.timeout(timeout),
        headers: { accept: "image/*,*/*;q=0.8" },
        redirect: "follow",
      });
    } catch (error) {
      const name = error instanceof Error ? error.name : "";
      throw new ImageError(
        name === "TimeoutError" || name === "AbortError" ? 504 : 502,
        "upstream image fetch failed",
      );
    }
    if (!response.ok) {
      await response.body?.cancel().catch(() => {});
      throw new ImageError(
        response.status >= 400 && response.status <= 599
          ? response.status
          : 502,
        "upstream image response failed",
      );
    }
    return {
      bytes: new Uint8Array(await response.arrayBuffer()),
      contentType:
        response.headers.get("content-type")?.split(";")[0]?.trim() ||
        undefined,
      maxAge: getMaxAge(response.headers.get("cache-control")),
    };
  };

  const optimize = async (params: ValidatedParams): Promise<CacheEntry> => {
    const source = await fetchSource(params);
    const upstreamType = detectContentType(source.bytes) ?? source.contentType;
    const maxAge = Math.max(images.minimumCacheTTL, source.maxAge);
    const finish = (
      bytes: Uint8Array,
      contentType: string,
      passthrough: boolean,
    ): CacheEntry => ({
      bytes,
      contentType,
      maxAge,
      expires: now() + maxAge * 1000,
      etag: `"${hash(bytes).slice(0, 32)}"`,
      passthrough,
    });
    if (upstreamType === SVG) {
      if (!images.dangerouslyAllowSVG)
        throw new ImageError(
          400,
          '"url" parameter is valid but image type is not allowed',
        );
      return finish(source.bytes, SVG, false);
    }
    if (
      upstreamType === ICO ||
      upstreamType === BMP ||
      isAnimated(source.bytes, upstreamType)
    )
      return finish(
        source.bytes,
        upstreamType ?? "application/octet-stream",
        false,
      );
    const format: ResizeFormat =
      params.mimeType ??
      (upstreamType === PNG ||
      upstreamType === JPEG ||
      upstreamType === WEBP ||
      upstreamType === AVIF
        ? upstreamType
        : undefined);
    let result: ResizeResult;
    try {
      result = await resize(source.bytes, {
        width: params.width,
        quality: params.quality,
        format,
      });
    } catch (error) {
      console.error(error);
      throw new ImageError(500, "image optimization failed");
    }
    return finish(
      result.bytes,
      result.contentType,
      result.passthrough === true,
    );
  };

  const respond = (
    request: Request,
    params: ValidatedParams,
    entry: CacheEntry,
    cache: "HIT" | "MISS",
  ): Response => {
    const headers = new Headers({
      "content-type": entry.contentType,
      "cache-control": `public, max-age=${entry.maxAge}, must-revalidate`,
      vary: "Accept",
      etag: entry.etag,
      "x-yolo-cache": cache,
      "content-security-policy": images.contentSecurityPolicy,
      "content-disposition": `${images.contentDispositionType}; filename="${fileName(params.href, entry.contentType)}"`,
    });
    if (entry.passthrough) headers.set("x-yolo-image", "passthrough");
    if (request.headers.get("if-none-match") === entry.etag)
      return new Response(null, { status: 304, headers });
    headers.set("content-length", String(entry.bytes.byteLength));
    return new Response(
      request.method === "HEAD"
        ? null
        : (entry.bytes as Uint8Array<ArrayBuffer>),
      {
        status: 200,
        headers,
      },
    );
  };

  const handle = async (request: Request): Promise<Response> => {
    if (request.method !== "GET" && request.method !== "HEAD")
      return new Response("Method Not Allowed", {
        status: 405,
        headers: { allow: "GET, HEAD" },
      });
    const url = new URL(request.url);
    if (!matches(url.pathname))
      return new Response("Not Found", { status: 404 });
    let params: ValidatedParams;
    try {
      params = validateParams(
        images,
        url.searchParams,
        request.headers.get("accept") ?? "",
      );
    } catch (error) {
      if (error instanceof ImageError)
        return new Response(error.message, { status: error.status });
      throw error;
    }
    const key = hash(
      `${params.href}\0${params.width}\0${params.quality}\0${params.mimeType ?? ""}`,
    );
    const time = now();
    const hit = memory.get(key, time) ?? (await disk?.get(key, time));
    if (hit) {
      memory.set(key, hit);
      return respond(request, params, hit, "HIT");
    }
    try {
      let pending = inflight.get(key);
      if (!pending) {
        pending = optimize(params);
        inflight.set(key, pending);
        const release = () => inflight.delete(key);
        pending.then(release, release);
      }
      const entry = await pending;
      memory.set(key, entry);
      await disk?.set(key, entry);
      return respond(request, params, entry, "MISS");
    } catch (error) {
      if (error instanceof ImageError)
        return new Response(error.message, { status: error.status });
      console.error(error);
      return new Response("Internal Server Error", { status: 500 });
    }
  };

  return { handle, matches };
}
