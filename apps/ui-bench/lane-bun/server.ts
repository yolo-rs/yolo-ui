// Lane 1: Bun.serve with HTML imports. Each page is a client-rendered React 19 tree built
// from the kit's real components; Bun bundles the HTML, TSX and Tailwind CSS (bun-plugin-tailwind,
// see ../bunfig.toml) on first request and serves the result from memory.
import rgGallery from "./rosegriffon/gallery.html";
import rgHome from "./rosegriffon/home.html";
import rgLegal from "./rosegriffon/legal.html";
import shGallery from "./shenron/gallery.html";
import shHome from "./shenron/home.html";
import shLegal from "./shenron/legal.html";

const port = Number(process.env.PORT ?? 4100);
const development = process.env.BENCH_DEV === "1";

const server = Bun.serve({
  port,
  hostname: process.env.HOST ?? "127.0.0.1",
  development,
  routes: {
    "/rosegriffon/": rgHome,
    "/rosegriffon/gallery": rgGallery,
    "/rosegriffon/legal": rgLegal,
    "/shenron/": shHome,
    "/shenron/gallery": shGallery,
    "/shenron/legal": shLegal,
    "/health": new Response("ok"),
  },
  fetch() {
    return new Response("Not found", { status: 404 });
  },
});

console.log(`bun lane listening on ${server.url} (development=${development}, bun ${Bun.version})`);
