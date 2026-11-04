// SPDX-License-Identifier: MIT OR Apache-2.0
// The image CDN layer: loader URLs, srcset generation, the `/_next/image` optimizer (validation, remote patterns,
// SVG policy, pass-through, caching, headers) and the `next/image` component attributes.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { deflateSync } from "node:zlib";
import { getImageProps } from "../src/app/image";
import { defaultConfig, resolveNextConfig } from "../src/config/next-config";
import type { ResolvedImages } from "../src/config/next-config";
import {
  createImageOptimizer,
  generateImgAttrs,
  getWidths,
  IMAGE_CONFIG_GLOBAL,
  imageConfigDefine,
  imageLoaderUrl,
  passthroughResizer,
} from "../src/image";
import type { Resizer } from "../src/image";

// A minimal PNG encoder: one 8-bit RGBA IDAT of a solid colour.
function crc32(bytes: Uint8Array): number {
  let crc = ~0;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
  }
  return ~crc >>> 0;
}

function chunk(type: string, data: Uint8Array): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  out.set(new TextEncoder().encode(type), 4);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
  return out;
}

function png(width: number, height: number): Uint8Array<ArrayBuffer> {
  const header = new Uint8Array(13);
  const view = new DataView(header.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  header.set([8, 6, 0, 0, 0], 8);
  const raw = new Uint8Array(height * (1 + width * 4));
  for (let y = 0; y < height; y++) {
    const row = y * (1 + width * 4);
    for (let x = 0; x < width; x++) raw.set([200, 30, 30, 255], row + 1 + x * 4);
  }
  const idat = new Uint8Array(deflateSync(raw));
  const parts = [
    new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", idat),
    chunk("IEND", new Uint8Array(0)),
  ];
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

const SVG_SOURCE =
  '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>';
const PNG_BYTES = png(8, 8);
const DEFAULTS = defaultConfig().images!;
const SIZES = {
  deviceSizes: DEFAULTS.deviceSizes!,
  imageSizes: DEFAULTS.imageSizes!,
};

async function images(overrides: Partial<ResolvedImages> = {}): Promise<ResolvedImages> {
  const resolved = await resolveNextConfig({}, "phase-test");
  return { ...resolved.images, ...overrides };
}

describe("image loaders", () => {
  const props = { src: "/photo.jpg", width: 640, quality: 80 };
  test("default and vercel", () => {
    expect(imageLoaderUrl({ loader: "default", path: "/_next/image" }, props)).toBe(
      "/_next/image?url=%2Fphoto.jpg&w=640&q=80",
    );
    expect(imageLoaderUrl({ loader: "default", path: "/_next/image" }, { src: "/a.png", width: 16 })).toBe(
      "/_next/image?url=%2Fa.png&w=16&q=75",
    );
    expect(imageLoaderUrl({ loader: "vercel", path: "/_next/image" }, props)).toBe(
      "/_vercel/image?url=%2Fphoto.jpg&w=640&q=80",
    );
  });
  test("imgix", () => {
    expect(imageLoaderUrl({ loader: "imgix", path: "https://example.imgix.net/" }, props)).toBe(
      "https://example.imgix.net/photo.jpg?auto=format&fit=max&w=640&q=80",
    );
  });
  test("cloudinary", () => {
    expect(
      imageLoaderUrl(
        {
          loader: "cloudinary",
          path: "https://res.cloudinary.com/demo/image/upload/",
        },
        props,
      ),
    ).toBe("https://res.cloudinary.com/demo/image/upload/f_auto,c_limit,w_640,q_80/photo.jpg");
    expect(imageLoaderUrl({ loader: "cloudinary", path: "https://c.example/" }, { src: "a.jpg", width: 100 })).toBe(
      "https://c.example/f_auto,c_limit,w_100,q_auto/a.jpg",
    );
  });
  test("akamai", () => {
    expect(imageLoaderUrl({ loader: "akamai", path: "https://example.com/" }, props)).toBe(
      "https://example.com/photo.jpg?imwidth=640",
    );
  });
  test("cloudflare", () => {
    expect(imageLoaderUrl({ loader: "cloudflare", path: "https://example.com" }, props)).toBe(
      "https://example.com/cdn-cgi/image/width=640,quality=80,format=auto/photo.jpg",
    );
  });
  test("fastly", () => {
    expect(imageLoaderUrl({ loader: "fastly", path: "https://example.com/" }, props)).toBe(
      "https://example.com/photo.jpg?auto=webp&width=640&quality=80",
    );
  });
  test("custom without a loader throws", () => {
    expect(() => imageLoaderUrl({ loader: "custom", path: "/" }, props)).toThrow(/loader/);
  });
});

describe("srcset", () => {
  test("sizes=100vw uses every device size", () => {
    expect(getWidths(SIZES, 400, "100vw")).toEqual({
      widths: DEFAULTS.deviceSizes!,
      kind: "w",
    });
    const attrs = generateImgAttrs({
      config: { loader: "default", path: "/_next/image", ...SIZES },
      src: "/a.jpg",
      width: 400,
      sizes: "100vw",
    });
    expect(attrs.sizes).toBe("100vw");
    expect(attrs.srcSet!.split(", ")).toHaveLength(8);
    expect(attrs.srcSet!.startsWith("/_next/image?url=%2Fa.jpg&w=640&q=75 640w")).toBe(true);
    expect(attrs.src).toBe("/_next/image?url=%2Fa.jpg&w=3840&q=75");
  });
  test("sizes with a small vw filters small widths", () => {
    const { widths } = getWidths(SIZES, undefined, "(max-width: 600px) 50vw, 25vw");
    expect(widths[0]).toBe(256);
    expect(widths).not.toContain(128);
  });
  test("fixed width snaps to 1x and 2x", () => {
    expect(getWidths(SIZES, 400)).toEqual({ widths: [640, 828], kind: "x" });
    const attrs = generateImgAttrs({
      config: { loader: "default", path: "/_next/image", ...SIZES },
      src: "/a.jpg",
      width: 400,
      quality: 50,
    });
    expect(attrs.srcSet).toBe("/_next/image?url=%2Fa.jpg&w=640&q=50 1x, /_next/image?url=%2Fa.jpg&w=828&q=50 2x");
    expect(attrs.src).toBe("/_next/image?url=%2Fa.jpg&w=828&q=50");
    expect(attrs.sizes).toBeUndefined();
  });
  test("no width means device sizes", () => {
    expect(getWidths(SIZES, undefined).kind).toBe("w");
    expect(
      generateImgAttrs({
        config: { loader: "default", path: "/_next/image", ...SIZES },
        src: "/a.jpg",
      }).sizes,
    ).toBe("100vw");
  });
  test("unoptimized keeps src", () => {
    expect(
      generateImgAttrs({
        config: { loader: "default", path: "/_next/image", ...SIZES },
        src: "/a.jpg",
        width: 10,
        unoptimized: true,
      }),
    ).toEqual({
      src: "/a.jpg",
      srcSet: undefined,
      sizes: undefined,
    });
  });
});

describe("optimizer", () => {
  let server: ReturnType<typeof Bun.serve>;
  let origin: string;
  let cacheDir: string;
  let fetched = 0;
  const local = async (pathname: string) => {
    if (pathname === "/local.png") return new Blob([PNG_BYTES], { type: "image/png" });
    if (pathname === "/icon.svg") return new Blob([SVG_SOURCE], { type: "image/svg+xml" });
    return undefined;
  };
  const resize: Resizer = async (bytes, opts) => ({
    bytes: bytes.subarray(0, 16),
    contentType: opts.format ?? "image/png",
  });

  beforeAll(() => {
    cacheDir = mkdtempSync(join(tmpdir(), "yolo-image-"));
    server = Bun.serve({
      port: 0,
      fetch(request: Request) {
        const { pathname } = new URL(request.url);
        fetched++;
        if (pathname === "/photo.png")
          return new Response(PNG_BYTES, {
            headers: {
              "content-type": "image/png",
              "cache-control": "public, max-age=86400",
            },
          });
        if (pathname === "/icon.svg")
          return new Response(SVG_SOURCE, {
            headers: { "content-type": "image/svg+xml" },
          });
        if (pathname === "/teapot.png") return new Response("no", { status: 418 });
        if (pathname === "/broken.png") return new Response("no", { status: 503 });
        return new Response("Not Found", { status: 404 });
      },
    });
    origin = `http://127.0.0.1:${server.port}`;
  });
  afterAll(() => {
    server.stop(true);
    rmSync(cacheDir, { recursive: true, force: true });
  });

  const get = (
    optimizer: ReturnType<typeof createImageOptimizer>,
    query: string,
    headers: Record<string, string> = {},
  ) => optimizer.handle(new Request(`http://app.test/_next/image?${query}`, { headers }));

  test("matches the configured and the yolo path", async () => {
    const optimizer = createImageOptimizer({ images: await images(), local });
    expect(optimizer.matches("/_next/image")).toBe(true);
    expect(optimizer.matches("/_yolo/image")).toBe(true);
    expect(optimizer.matches("/_next/images")).toBe(false);
    const custom = createImageOptimizer({
      images: await images({ path: "/cdn/img" }),
      local,
    });
    expect(custom.matches("/cdn/img")).toBe(true);
    expect(custom.matches("/_next/image")).toBe(false);
  });

  test("validates url, w and q", async () => {
    const optimizer = createImageOptimizer({
      images: await images(),
      local,
      resize,
    });
    let response = await get(optimizer, "w=640&q=75");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain('"url"');
    response = await get(optimizer, "url=%2Flocal.png&q=75");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain('"w"');
    response = await get(optimizer, "url=%2Flocal.png&w=500&q=75");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain("500 is not allowed");
    response = await get(optimizer, "url=%2Flocal.png&w=640");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain('"q"');
    response = await get(optimizer, "url=%2Flocal.png&w=640&q=80");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain("80 is not allowed");
    response = await get(optimizer, "url=%2F%2Fevil.test%2Fa.png&w=640&q=75");
    expect(response.status).toBe(400);
    response = await get(optimizer, "url=%2F_next%2Fimage%3Furl%3D%2Fa.png&w=640&q=75");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain("recursive");
    response = await get(optimizer, "url=ftp%3A%2F%2Fx%2Fa.png&w=640&q=75");
    expect(response.status).toBe(400);
    response = await get(optimizer, "url=%2Fmissing.png&w=640&q=75");
    expect(response.status).toBe(404);
  });

  test("local patterns restrict relative urls", async () => {
    const optimizer = createImageOptimizer({
      images: await images({ localPatterns: [{ pathname: "/assets/**" }] }),
      local,
      resize,
    });
    expect((await get(optimizer, "url=%2Flocal.png&w=640&q=75")).status).toBe(400);
  });

  test("remote urls must match remotePatterns", async () => {
    const optimizer = createImageOptimizer({
      images: await images({
        remotePatterns: [
          {
            protocol: "http",
            hostname: "127.0.0.1",
            port: String(server.port),
            pathname: "/**",
          },
        ],
      }),
      local,
      resize,
    });
    const denied = await get(optimizer, `url=${encodeURIComponent("http://evil.test/photo.png")}&w=640&q=75`);
    expect(denied.status).toBe(400);
    expect(await denied.text()).toContain("not allowed");
    const wrongPort = await get(
      optimizer,
      `url=${encodeURIComponent(`http://127.0.0.1:${(server.port ?? 0) + 1}/photo.png`)}&w=640&q=75`,
    );
    expect(wrongPort.status).toBe(400);
    const allowed = await get(optimizer, `url=${encodeURIComponent(`${origin}/photo.png`)}&w=640&q=75`, {
      accept: "image/avif,image/webp,*/*",
    });
    expect(allowed.status).toBe(200);
    expect(allowed.headers.get("content-type")).toBe("image/webp");
    expect(allowed.headers.get("x-yolo-cache")).toBe("MISS");
    expect(allowed.headers.get("vary")).toBe("Accept");
    // Upstream max-age (86400) is larger than minimumCacheTTL (14400).
    expect(allowed.headers.get("cache-control")).toBe("public, max-age=86400, must-revalidate");
    expect(allowed.headers.get("content-disposition")).toBe('attachment; filename="photo.webp"');
    expect(new Uint8Array(await allowed.arrayBuffer())).toEqual(PNG_BYTES.subarray(0, 16));
  });

  test("wildcard hostnames", async () => {
    const optimizer = createImageOptimizer({
      images: await images({
        remotePatterns: [{ hostname: "**.example.com" }, { hostname: "*.one.test" }],
      }),
      local,
      resize,
    });
    const status = async (url: string) => (await get(optimizer, `url=${encodeURIComponent(url)}&w=640&q=75`)).status;
    // 404s come from the (unreachable) fetch, so a non-400 status proves the pattern matched.
    expect(await status("https://example.org/a.png")).toBe(400);
    expect(await status("https://b.a.one.test/a.png")).toBe(400);
    expect(await status("https://evil.one.test.attacker/a.png")).toBe(400);
  });

  test("upstream errors keep their status", async () => {
    const optimizer = createImageOptimizer({
      images: await images({ remotePatterns: [{ hostname: "127.0.0.1" }] }),
      local,
      resize,
    });
    expect((await get(optimizer, `url=${encodeURIComponent(`${origin}/teapot.png`)}&w=640&q=75`)).status).toBe(418);
    expect((await get(optimizer, `url=${encodeURIComponent(`${origin}/broken.png`)}&w=640&q=75`)).status).toBe(503);
    expect((await get(optimizer, `url=${encodeURIComponent(`${origin}/nope.png`)}&w=640&q=75`)).status).toBe(404);
  });

  test("svg is refused unless dangerouslyAllowSVG", async () => {
    const refused = createImageOptimizer({
      images: await images(),
      local,
      resize,
    });
    const response = await get(refused, "url=%2Ficon.svg&w=640&q=75");
    expect(response.status).toBe(400);
    expect(await response.text()).toContain("image type is not allowed");
    const allowed = createImageOptimizer({
      images: await images({
        dangerouslyAllowSVG: true,
        contentDispositionType: "inline",
      }),
      local,
      resize,
    });
    const ok = await get(allowed, "url=%2Ficon.svg&w=640&q=75", {
      accept: "image/webp",
    });
    expect(ok.status).toBe(200);
    expect(ok.headers.get("content-type")).toBe("image/svg+xml");
    expect(ok.headers.get("content-security-policy")).toBe("script-src 'none'; frame-src 'none'; sandbox;");
    expect(ok.headers.get("content-disposition")).toBe('inline; filename="icon.svg"');
    expect(await ok.text()).toBe(SVG_SOURCE);
  });

  test("passthrough when there is no resizer", async () => {
    const optimizer = createImageOptimizer({
      images: await images(),
      local,
      resize: passthroughResizer,
    });
    const response = await get(optimizer, "url=%2Flocal.png&w=640&q=75", {
      accept: "image/webp",
    });
    expect(response.status).toBe(200);
    expect(response.headers.get("x-yolo-image")).toBe("passthrough");
    expect(response.headers.get("content-type")).toBe("image/png");
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(PNG_BYTES);
  });

  test("default resizer serves the image (sharp or pass-through)", async () => {
    const optimizer = createImageOptimizer({ images: await images(), local });
    const response = await get(optimizer, "url=%2Flocal.png&w=16&q=75", {
      accept: "image/webp",
    });
    expect(response.status).toBe(200);
    const passthrough = response.headers.get("x-yolo-image") === "passthrough";
    expect(response.headers.get("content-type")).toBe(passthrough ? "image/png" : "image/webp");
    expect((await response.arrayBuffer()).byteLength).toBeGreaterThan(0);
  });

  test("content negotiation", async () => {
    const optimizer = createImageOptimizer({
      images: await images({ formats: ["image/avif", "image/webp"] }),
      local,
      resize,
    });
    const type = async (accept: string) =>
      (await get(optimizer, "url=%2Flocal.png&w=640&q=75", { accept })).headers.get("content-type");
    expect(await type("image/avif,image/webp,*/*")).toBe("image/avif");
    expect(await type("image/webp,*/*")).toBe("image/webp");
    expect(await type("image/jpeg")).toBe("image/png");
  });

  test("memory and disk cache hit on the second request", async () => {
    let clock = 1_000_000;
    const now = () => clock;
    const before = fetched;
    const remote = `url=${encodeURIComponent(`${origin}/photo.png`)}&w=640&q=75`;
    const first = createImageOptimizer({
      images: await images({ remotePatterns: [{ hostname: "127.0.0.1" }] }),
      local,
      resize,
      cacheDir,
      now,
    });
    const miss = await get(first, remote, { accept: "image/webp" });
    expect(miss.headers.get("x-yolo-cache")).toBe("MISS");
    const hit = await get(first, remote, { accept: "image/webp" });
    expect(hit.headers.get("x-yolo-cache")).toBe("HIT");
    expect(hit.headers.get("cache-control")).toBe("public, max-age=86400, must-revalidate");
    expect(hit.headers.get("vary")).toBe("Accept");
    expect(await hit.arrayBuffer()).toEqual(await miss.arrayBuffer());
    expect(fetched - before).toBe(1);
    // A different Accept is a different variant.
    expect((await get(first, remote, { accept: "image/jpeg" })).headers.get("x-yolo-cache")).toBe("MISS");
    // A new process with the same cacheDir reads the disk entry.
    const second = createImageOptimizer({
      images: await images({ remotePatterns: [{ hostname: "127.0.0.1" }] }),
      local,
      resize,
      cacheDir,
      now,
    });
    expect((await get(second, remote, { accept: "image/webp" })).headers.get("x-yolo-cache")).toBe("HIT");
    expect(fetched - before).toBe(2);
    // Expired entries are fetched again.
    clock += 86400 * 1000 + 1;
    expect((await get(second, remote, { accept: "image/webp" })).headers.get("x-yolo-cache")).toBe("MISS");
    expect(fetched - before).toBe(3);
  });

  test("etag and HEAD", async () => {
    const optimizer = createImageOptimizer({
      images: await images(),
      local,
      resize,
    });
    const response = await get(optimizer, "url=%2Flocal.png&w=640&q=75");
    const etag = response.headers.get("etag")!;
    expect(etag).toMatch(/^"[0-9a-f]+"$/);
    const notModified = await get(optimizer, "url=%2Flocal.png&w=640&q=75", {
      "if-none-match": etag,
    });
    expect(notModified.status).toBe(304);
    const head = await optimizer.handle(
      new Request("http://app.test/_yolo/image?url=%2Flocal.png&w=640&q=75", {
        method: "HEAD",
      }),
    );
    expect(head.status).toBe(200);
    expect(head.headers.get("content-length")).toBe("16");
    expect(await head.arrayBuffer()).toHaveLength(0);
    const post = await optimizer.handle(
      new Request("http://app.test/_next/image?url=%2Flocal.png&w=640&q=75", {
        method: "POST",
      }),
    );
    expect(post.status).toBe(405);
  });
});

describe("next/image", () => {
  const g = globalThis as { __YOLO_IMAGE_CONFIG__?: unknown };
  afterAll(() => {
    delete g.__YOLO_IMAGE_CONFIG__;
  });

  test("imageConfigDefine", async () => {
    const define = imageConfigDefine(await images({ loaderFile: "./loader.ts", loader: "custom" }));
    expect(Object.keys(define)).toEqual([IMAGE_CONFIG_GLOBAL]);
    expect(JSON.parse(define[IMAGE_CONFIG_GLOBAL]!)).toEqual({
      loader: "custom",
      path: "/_next/image",
      deviceSizes: DEFAULTS.deviceSizes,
      imageSizes: DEFAULTS.imageSizes,
      qualities: [75],
      unoptimized: false,
      loaderFile: true,
    });
  });

  test("fixed size image", () => {
    delete g.__YOLO_IMAGE_CONFIG__;
    const { props } = getImageProps({
      src: "/hero.jpg",
      alt: "Hero",
      width: 400,
      height: 300,
      quality: 75,
    });
    expect(props.src).toBe("/_next/image?url=%2Fhero.jpg&w=828&q=75");
    expect(props.srcSet).toBe("/_next/image?url=%2Fhero.jpg&w=640&q=75 1x, /_next/image?url=%2Fhero.jpg&w=828&q=75 2x");
    expect(props.width).toBe(400);
    expect(props.height).toBe(300);
    expect(props.loading).toBe("lazy");
    expect(props.decoding).toBe("async");
    expect(props.fetchPriority).toBeUndefined();
    expect(props.onLoad).toBeUndefined();
    expect(props.style).toEqual({ color: "transparent" });
  });

  test("priority, fill, sizes and overrideSrc", () => {
    const { props } = getImageProps({
      src: "/hero.jpg",
      alt: "",
      fill: true,
      sizes: "50vw",
      priority: true,
      overrideSrc: "/hero.jpg",
      style: { objectFit: "contain" },
    });
    expect(props.loading).toBe("eager");
    expect(props.fetchPriority).toBe("high");
    expect(props.src).toBe("/hero.jpg");
    expect(props.sizes).toBe("50vw");
    expect(props.srcSet!.split(", ").map(c => c.split(" ")[1])).toEqual([
      "384w",
      "640w",
      "750w",
      "828w",
      "1080w",
      "1200w",
      "1920w",
      "2048w",
      "3840w",
    ]);
    expect(props.width).toBeUndefined();
    expect(props.style).toMatchObject({
      position: "absolute",
      width: "100%",
      height: "100%",
      objectFit: "contain",
    });
  });

  test("static import, blur placeholder and onLoad", () => {
    const data = {
      src: "/media/a-1234.png",
      width: 800,
      height: 400,
      blurDataURL: "data:image/png;base64,AAAA",
    };
    const loaded: string[] = [];
    const { props } = getImageProps({
      src: data,
      alt: "a",
      placeholder: "blur",
      width: 200,
      onLoad: () => loaded.push("load"),
    });
    expect(props.width).toBe(200);
    expect(props.height).toBe(100);
    expect(props.style).toMatchObject({
      backgroundImage: 'url("data:image/png;base64,AAAA")',
      backgroundSize: "cover",
    });
    expect(typeof props.onLoad).toBe("function");
    const style: Record<string, string> = { backgroundImage: "x" };
    props.onLoad!({ currentTarget: { style } } as never);
    expect(loaded).toEqual(["load"]);
    expect(style.backgroundImage).toBe("none");
  });

  test("unoptimized, data urls and svg keep src", () => {
    expect(getImageProps({ src: "/logo.svg", alt: "", width: 10, height: 10 }).props.src).toBe("/logo.svg");
    expect(
      getImageProps({
        src: "data:image/png;base64,AAAA",
        alt: "",
        width: 10,
        height: 10,
      }).props.srcSet,
    ).toBeUndefined();
    expect(
      getImageProps({
        src: "/a.png",
        alt: "",
        width: 10,
        height: 10,
        unoptimized: true,
      }).props.src,
    ).toBe("/a.png");
  });

  test("loader prop and runtime config", () => {
    const loader = ({ src, width }: { src: string; width: number }) => `https://cdn.test${src}?w=${width}`;
    expect(getImageProps({ src: "/a.png", alt: "", width: 100, height: 100, loader }).props.src).toBe(
      "https://cdn.test/a.png?w=256",
    );
    g.__YOLO_IMAGE_CONFIG__ = {
      loader: "cloudinary",
      path: "https://res.cloudinary.com/demo/image/upload/",
      deviceSizes: [320, 640],
      imageSizes: [16],
    };
    const { props } = getImageProps({
      src: "/a.png",
      alt: "",
      width: 300,
      height: 100,
    });
    expect(props.src).toBe("https://res.cloudinary.com/demo/image/upload/f_auto,c_limit,w_640,q_auto/a.png");
    expect(props.srcSet).toBe(
      "https://res.cloudinary.com/demo/image/upload/f_auto,c_limit,w_320,q_auto/a.png 1x, https://res.cloudinary.com/demo/image/upload/f_auto,c_limit,w_640,q_auto/a.png 2x",
    );
    g.__YOLO_IMAGE_CONFIG__ = { loader: "custom" };
    expect(() => getImageProps({ src: "/a.png", alt: "", width: 1, height: 1 })).toThrow(/custom/);
    delete g.__YOLO_IMAGE_CONFIG__;
  });

  test("missing dimensions throw", () => {
    expect(() => getImageProps({ src: "/a.png", alt: "" })).toThrow(/width/);
    expect(() => getImageProps({ src: "/a.png", alt: "", fill: true, width: 10 })).toThrow(/fill/);
    expect(() => getImageProps({ src: "//cdn.test/a.png", alt: "", width: 1, height: 1 })).toThrow(/protocol-relative/);
  });
});
