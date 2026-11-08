// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { deflateSync } from "node:zlib";
import {
  akamaiLoader,
  cloudflareLoader,
  cloudfrontLoader,
  cloudinaryLoader,
  coerceQuality,
  createImageOptimizerHandler,
  createImagePolicy,
  defaultLoader,
  describeImageBackend,
  detectImageType,
  fastlyLoader,
  getImageProps,
  imgixLoader,
  isAnimatedImage,
  isPrivateHost,
  loaderFor,
  localPatternMatches,
  matchesImagePolicy,
  negotiateImageFormat,
  parseMaxAge,
  remotePatternMatches,
  supabaseLoader,
} from "../src/image.ts";
import type { ImageOptimizerPolicy } from "../src/types.ts";

function crc32(bytes: Uint8Array): number {
  let crc = ~0;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return ~crc >>> 0;
}

function pngChunk(type: string, data: Uint8Array): Uint8Array {
  const typeBytes = new TextEncoder().encode(type);
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  out.set(typeBytes, 4);
  out.set(data, 8);
  const crcInput = new Uint8Array(4 + data.length);
  crcInput.set(typeBytes);
  crcInput.set(data, 4);
  view.setUint32(8 + data.length, crc32(crcInput));
  return out;
}

/** Builds a valid RGB PNG of the requested size. */
export function syntheticPng(width: number, height: number): Uint8Array {
  const header = new Uint8Array(13);
  const view = new DataView(header.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  header[8] = 8;
  header[9] = 2;
  const stride = width * 3 + 1;
  const raw = new Uint8Array(stride * height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = y * stride + 1 + x * 3;
      raw[offset] = (x * 7) & 255;
      raw[offset + 1] = (y * 13) & 255;
      raw[offset + 2] = 128;
    }
  }
  const parts = [
    new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk("IHDR", header),
    pngChunk("IDAT", new Uint8Array(deflateSync(raw))),
    pngChunk("IEND", new Uint8Array(0)),
  ];
  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

const fixturePolicy: ImageOptimizerPolicy = createImagePolicy({
  formats: ["image/avif", "image/webp"],
  qualities: [50, 75, 90],
  minimumCacheTtl: 86_400,
  remotePatterns: [
    { protocol: "https", hostname: "images.unsplash.com", pathname: "/**" },
    { protocol: "https", hostname: "**.cloudfront.net" },
    { protocol: "https", hostname: "cdn.example.com", port: "", pathname: "/media/**", search: "" },
  ],
  localPatterns: [{ pathname: "/assets/images/**", search: "" }],
});

describe("remote and local patterns", () => {
  test("hostname wildcards", () => {
    const deep = { protocol: "https" as const, hostname: "**.cloudfront.net" };
    expect(remotePatternMatches(deep, "https://a.b.cloudfront.net/x.png")).toBe(true);
    expect(remotePatternMatches(deep, "https://a.cloudfront.net/x.png")).toBe(true);
    expect(remotePatternMatches(deep, "https://cloudfront.net/x.png")).toBe(false);
    expect(remotePatternMatches(deep, "http://a.cloudfront.net/x.png")).toBe(false);
    const single = { hostname: "*.x.com" };
    expect(remotePatternMatches(single, "https://a.x.com/")).toBe(true);
    expect(remotePatternMatches(single, "https://a.b.x.com/")).toBe(false);
    expect(remotePatternMatches(single, "https://x.com/")).toBe(false);
    expect(remotePatternMatches({ hostname: "Example.COM" }, "https://example.com/a")).toBe(true);
  });

  test("pathname globs, port and search", () => {
    const account = { hostname: "example.com", pathname: "/account123/**" };
    expect(remotePatternMatches(account, "https://example.com/account123/a/b.png")).toBe(true);
    expect(remotePatternMatches(account, "https://example.com/account/a.png")).toBe(false);
    expect(remotePatternMatches({ hostname: "example.com", pathname: "/img/*" }, "https://example.com/img/a.png")).toBe(true);
    expect(remotePatternMatches({ hostname: "example.com", pathname: "/img/*" }, "https://example.com/img/a/b.png")).toBe(false);
    expect(remotePatternMatches({ hostname: "cdn.example.com", port: "" }, "https://cdn.example.com/a")).toBe(true);
    expect(remotePatternMatches({ hostname: "cdn.example.com", port: "" }, "https://cdn.example.com:8080/a")).toBe(false);
    expect(remotePatternMatches({ hostname: "cdn.example.com", port: "8080" }, "https://cdn.example.com:8080/a")).toBe(true);
    expect(remotePatternMatches({ hostname: "cdn.example.com", search: "" }, "https://cdn.example.com/a?v=1")).toBe(false);
    expect(remotePatternMatches({ hostname: "cdn.example.com", search: "?v=1" }, "https://cdn.example.com/a?v=1")).toBe(true);
  });

  test("local patterns and policy matching", () => {
    const pattern = { pathname: "/assets/images/**", search: "" };
    expect(localPatternMatches(pattern, "/assets/images/a.png")).toBe(true);
    expect(localPatternMatches(pattern, "/assets/images/a.png?x=1")).toBe(false);
    expect(localPatternMatches(pattern, "/other.png")).toBe(false);
    expect(matchesImagePolicy(fixturePolicy, "https://images.unsplash.com/photo-1")).toBe(true);
    expect(matchesImagePolicy(fixturePolicy, "https://untrusted.example.net/x.png")).toBe(false);
    expect(matchesImagePolicy(fixturePolicy, "https://cdn.example.com/media/a.png")).toBe(true);
    expect(matchesImagePolicy(fixturePolicy, "https://cdn.example.com/other/a.png")).toBe(false);
    expect(matchesImagePolicy(fixturePolicy, "/assets/images/a.png")).toBe(true);
    expect(matchesImagePolicy(fixturePolicy, "/hero.png")).toBe(false);
    expect(matchesImagePolicy(createImagePolicy(), "/hero.png")).toBe(true);
  });
});

describe("loaders", () => {
  test("default loader", () => {
    expect(defaultLoader({ src: "https://images.unsplash.com/photo-1", width: 1080, quality: 90 })).toBe(
      "/_yolo/image?url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=1080&q=90",
    );
    expect(defaultLoader({ src: "/hero.png", width: 640 }, { route: "/_next/image", deploymentId: "dpl_1" })).toBe(
      "/_next/image?url=%2Fhero.png&w=640&q=75&dpl=dpl_1",
    );
    expect(defaultLoader({ src: "https://a.com/x.png", width: 640 }, { deploymentId: "dpl_1" })).not.toContain("dpl=");
  });

  test("CDN loaders", () => {
    expect(cloudflareLoader({ src: "/img/a.png", width: 640, quality: 80 })).toBe("/cdn-cgi/image/width=640,quality=80,format=auto/img/a.png");
    expect(cloudinaryLoader({ src: "https://res.cloudinary.com/demo/image/upload/sample.jpg", width: 800 })).toBe(
      "https://res.cloudinary.com/f_auto,c_limit,w_800,q_auto/demo/image/upload/sample.jpg",
    );
    expect(cloudinaryLoader({ src: "/sample.jpg", width: 800, quality: 60 })).toBe("/f_auto,c_limit,w_800,q_60/sample.jpg");
    const imgix = imgixLoader({ src: "https://ex.imgix.net/a.png?crop=faces", width: 400, quality: 60 });
    expect(imgix).toContain("auto=format");
    expect(imgix).toContain("fit=max");
    expect(imgix).toContain("w=400");
    expect(imgix).toContain("q=60");
    expect(imgix).toContain("crop=faces");
    expect(akamaiLoader({ src: "/a.png", width: 400 })).toBe("/a.png?imwidth=400");
    expect(fastlyLoader({ src: "/a.png", width: 400 })).toBe("/a.png?auto=webp&width=400&quality=75");
    expect(supabaseLoader({ src: "/storage/a.png", width: 400, quality: 60 })).toBe("/storage/a.png?width=400&quality=60");
    expect(cloudfrontLoader({ src: "https://d1.cloudfront.net/a.png", width: 400 })).toBe(
      "https://d1.cloudfront.net/a.png?format=auto&width=400&quality=75",
    );
    expect(loaderFor("unoptimized")({ src: "/a.png", width: 1 })).toBe("/a.png");
    expect(() => loaderFor("custom")).toThrow();
  });
});

describe("quality coercion", () => {
  test("nearest allowed quality with ties going lower", () => {
    expect(coerceQuality(fixturePolicy, 80)).toBe(75);
    expect(coerceQuality(fixturePolicy, 88)).toBe(90);
    expect(coerceQuality(fixturePolicy, 62)).toBe(50);
    expect(coerceQuality(fixturePolicy)).toBe(75);
    expect(coerceQuality(createImagePolicy({ qualities: [50, 70] }), 60)).toBe(50);
    expect(coerceQuality(createImagePolicy({ qualities: [60, 80] }))).toBe(60);
  });
});

describe("getImageProps", () => {
  test("sizes with viewport widths selects widths from 384 up", () => {
    const { props } = getImageProps(
      { src: "/hero.png", alt: "Hero", width: 1200, height: 800, sizes: "(max-width: 768px) 100vw, 50vw" },
      fixturePolicy,
    );
    const widths = (props.srcSet ?? "").split(", ").map((candidate) => candidate.split(" ")[1]);
    expect(widths).toEqual(["384w", "640w", "750w", "828w", "1080w", "1200w", "1920w", "2048w", "3840w"]);
    expect(props.sizes).toBe("(max-width: 768px) 100vw, 50vw");
    expect(props.loading).toBe("lazy");
    expect(props.decoding).toBe("async");
    expect(props.src).toBe("/_yolo/image?url=%2Fhero.png&w=3840&q=75");
    expect(props["data-nimg"]).toBe("1");
    expect(props.width).toBe(1200);
    expect(props.height).toBe(800);
  });

  test("fixed width emits 1x / 2x candidates", () => {
    const { props } = getImageProps(
      { src: "https://images.unsplash.com/photo-1", alt: "Remote", width: 800, height: 600, quality: 90 },
      fixturePolicy,
    );
    expect(props.srcSet).toBe(
      "/_yolo/image?url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=828&q=90 1x, /_yolo/image?url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=1920&q=90 2x",
    );
    expect(props.src).toBe("/_yolo/image?url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=1920&q=90");
    expect(props.sizes).toBeUndefined();
  });

  test("fill uses device sizes and 100vw", () => {
    const { props } = getImageProps({ src: "/hero.png", alt: "Hero", fill: true }, fixturePolicy);
    expect(props.sizes).toBe("100vw");
    expect((props.srcSet ?? "").split(", ")).toHaveLength(8);
    expect(props.style.position).toBe("absolute");
    expect(props.style.inset).toBe(0);
    expect(props.width).toBeUndefined();
    expect(props["data-nimg"]).toBe("fill");
  });

  test("unoptimized, svg and data URLs pass through", () => {
    expect(getImageProps({ src: "/a.png", alt: "", width: 10, height: 10, unoptimized: true }, fixturePolicy).props).toMatchObject({
      src: "/a.png",
      srcSet: undefined,
    });
    expect(getImageProps({ src: "/logo.svg", alt: "", width: 10, height: 10 }, fixturePolicy).props.src).toBe("/logo.svg");
    const data = getImageProps({ src: "data:image/png;base64,AAAA", alt: "", width: 10, height: 10 }, fixturePolicy).props;
    expect(data.src).toBe("data:image/png;base64,AAAA");
    expect(data.loading).toBe("eager");
  });

  test("priority, static imports and blur placeholder", () => {
    const hero = { src: "/_yolo/static/media/hero.png", width: 1200, height: 800, blurDataURL: "data:image/png;base64,AAA", blurWidth: 8, blurHeight: 5 };
    const { props, meta } = getImageProps({ src: hero, alt: "Hero", priority: true, placeholder: "blur" }, fixturePolicy);
    expect(props.loading).toBe("eager");
    expect(props.fetchPriority).toBe("high");
    expect(props.width).toBe(1200);
    expect(props.height).toBe(800);
    expect(props.style.backgroundSize).toBe("cover");
    expect(String(props.style.backgroundImage)).toStartWith('url("data:image/svg+xml;charset=utf-8,%3Csvg');
    expect(String(props.style.backgroundImage)).toContain("feGaussianBlur");
    expect(meta.priority).toBe(true);
    const scaled = getImageProps({ src: hero, alt: "Hero", width: 600 }, fixturePolicy).props;
    expect(scaled.height).toBe(400);
  });

  test("validation errors in development", () => {
    expect(() => getImageProps({ src: "/a.png", width: 1, height: 1 } as never, fixturePolicy)).toThrow(/missing required "alt"/);
    expect(() => getImageProps({ src: "/a.png", alt: "", height: 1 }, fixturePolicy)).toThrow(/missing required "width"/);
    expect(() => getImageProps({ src: "//cdn/x.png", alt: "", width: 1, height: 1 }, fixturePolicy)).toThrow(/protocol-relative/);
    expect(() => getImageProps({ src: "/a.png", alt: "", fill: true, width: 10 }, fixturePolicy)).toThrow(/both "width" and "fill"/);
    expect(() => getImageProps({ src: "/a.png", alt: "", width: 1, height: 1, placeholder: "blur" }, fixturePolicy)).toThrow(/blurDataURL/);
  });
});

describe("image helpers", () => {
  test("type detection, animation and private hosts", () => {
    expect(detectImageType(syntheticPng(4, 4))).toBe("image/png");
    expect(detectImageType(new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg"></svg>'))).toBe("image/svg+xml");
    expect(detectImageType(new Uint8Array([0xff, 0xd8, 0xff, 0xe0]))).toBe("image/jpeg");
    expect(isAnimatedImage(syntheticPng(4, 4), "image/png")).toBe(false);
    expect(isPrivateHost("10.1.2.3")).toBe(true);
    expect(isPrivateHost("172.20.0.1")).toBe(true);
    expect(isPrivateHost("172.32.0.1")).toBe(false);
    expect(isPrivateHost("192.168.1.1")).toBe(true);
    expect(isPrivateHost("127.0.0.1")).toBe(true);
    expect(isPrivateHost("169.254.1.1")).toBe(true);
    expect(isPrivateHost("[::1]")).toBe(true);
    expect(isPrivateHost("fd12::1")).toBe(true);
    expect(isPrivateHost("fe80::1")).toBe(true);
    expect(isPrivateHost("localhost")).toBe(true);
    expect(isPrivateHost("images.unsplash.com")).toBe(false);
    expect(isPrivateHost("8.8.8.8")).toBe(false);
  });

  test("format negotiation and cache-control parsing", () => {
    expect(negotiateImageFormat(fixturePolicy, "image/avif,image/webp,image/apng,*/*;q=0.8")).toBe("image/avif");
    expect(negotiateImageFormat(fixturePolicy, "image/webp,*/*")).toBe("image/webp");
    expect(negotiateImageFormat(fixturePolicy, "*/*")).toBeUndefined();
    expect(negotiateImageFormat(fixturePolicy, null)).toBeUndefined();
    expect(negotiateImageFormat(fixturePolicy, "image/avif;q=0,image/webp")).toBe("image/webp");
    expect(parseMaxAge("public, max-age=600")).toBe(600);
    expect(parseMaxAge("public, s-maxage=1200, max-age=600")).toBe(1200);
    expect(parseMaxAge(null)).toBeUndefined();
  });
});

describe("optimizer handler", () => {
  let root = "";
  const png = syntheticPng(64, 48);
  const policy = createImagePolicy({
    formats: ["image/webp"],
    qualities: [50, 75, 90],
    minimumCacheTtl: 86_400,
    maximumRedirects: 2,
    remotePatterns: [
      { protocol: "https", hostname: "images.unsplash.com", pathname: "/**" },
      { protocol: "https", hostname: "redirect.example.com" },
      { protocol: "http", hostname: "127.0.0.1" },
    ],
  });
  const requests: string[] = [];
  const fetchImpl: typeof fetch = (async (input: string | URL | Request) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
    requests.push(url);
    if (url.startsWith("https://redirect.example.com/")) {
      return new Response(null, { status: 302, headers: { location: "https://redirect.example.com/again" } });
    }
    if (url.startsWith("https://images.unsplash.com/")) {
      return new Response(png as BodyInit, { status: 200, headers: { "content-type": "image/png", "cache-control": "public, max-age=600" } });
    }
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
  let handler: (request: Request) => Promise<Response>;

  beforeAll(async () => {
    root = mkdtempSync(join(tmpdir(), "yolo-next-image-"));
    mkdirSync(join(root, "public"), { recursive: true });
    await Bun.write(join(root, "public", "photo.png"), png);
    handler = createImageOptimizerHandler(policy, { root, dev: false, fetchImpl });
  });

  afterAll(() => {
    rmSync(root, { recursive: true, force: true });
  });

  const call = (query: string, headers: Record<string, string> = {}) =>
    handler(new Request(`http://localhost/_yolo/image?${query}`, { headers }));

  test("validates url, w and q", async () => {
    expect(await (await call("w=640&q=75")).text()).toBe('"url" parameter is required');
    expect((await call("url=%2Fphoto.png&w=333&q=75")).status).toBe(400);
    expect(await (await call("url=%2Fphoto.png&w=333&q=75")).text()).toContain("not allowed");
    expect(await (await call("url=%2Fphoto.png&q=75")).text()).toBe('"w" parameter (width) is required');
    expect(await (await call("url=%2Fphoto.png&w=640")).text()).toBe('"q" parameter (quality) is required');
    expect(await (await call("url=%2Fphoto.png&w=640&q=80")).text()).toContain("not allowed");
    expect(await (await call("url=https%3A%2F%2Funtrusted.example.net%2Fx.png&w=640&q=75")).text()).toBe('"url" parameter is not allowed');
    expect((await call("url=%2F%2Fcdn%2Fx.png&w=640&q=75")).status).toBe(400);
    expect((await call("url=%2F_yolo%2Fimage%3Furl%3Dx&w=640&q=75")).status).toBe(400);
    expect((await call("url=http%3A%2F%2F127.0.0.1%2Fx.png&w=640&q=75")).status).toBe(400);
    expect((await call("url=%2Fmissing.png&w=640&q=75")).status).toBe(404);
  });

  test("serves a local file with the Next.js response headers, caches and supports 304", async () => {
    const first = await call("url=%2Fphoto.png&w=640&q=75", { accept: "image/webp,*/*" });
    expect(first.status).toBe(200);
    expect(first.headers.get("vary")).toBe("Accept");
    expect(first.headers.get("cache-control")).toBe("public, max-age=86400, must-revalidate");
    expect(first.headers.get("content-security-policy")).toBe(policy.contentSecurityPolicy);
    expect(first.headers.get("content-disposition")).toMatch(/^attachment; filename="photo\.(webp|png)"$/);
    expect(["image/webp", "image/png"]).toContain(first.headers.get("content-type") ?? "");
    expect(first.headers.get("x-nextjs-cache")).toBe("MISS");
    const etag = first.headers.get("etag");
    expect(etag).toMatch(/^"[0-9a-f]{32}"$/);
    const bytes = new Uint8Array(await first.arrayBuffer());
    expect(bytes.byteLength).toBeGreaterThan(0);
    if (describeImageBackend() === "bun-image") {
      expect(first.headers.get("content-type")).toBe("image/webp");
    }
    const second = await call("url=%2Fphoto.png&w=640&q=75", { accept: "image/webp,*/*" });
    expect(second.headers.get("x-nextjs-cache")).toBe("HIT");
    const notModified = await call("url=%2Fphoto.png&w=640&q=75", { accept: "image/webp,*/*", "if-none-match": etag ?? "" });
    expect(notModified.status).toBe(304);
  });

  test("fetches remote images through fetchImpl and honours upstream cache-control", async () => {
    const response = await call("url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=640&q=90", { accept: "image/webp" });
    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("public, max-age=86400, must-revalidate");
    expect(requests.some((url) => url.startsWith("https://images.unsplash.com/photo-1"))).toBe(true);
    const redirected = await call("url=https%3A%2F%2Fredirect.example.com%2Fstart&w=640&q=75");
    expect(redirected.status).toBe(508);
  });

  test("reports the transformation backend", () => {
    expect(["bun-image", "sharp", "passthrough"]).toContain(describeImageBackend());
  });
});
