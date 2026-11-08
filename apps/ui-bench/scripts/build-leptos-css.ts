// Compiles the Leptos lane stylesheets (kit tokens + utilities used by the Leptos markup) with
// Bun's bundler and bun-plugin-tailwind. Output: dist/leptos-css/{rosegriffon,shenron}.css.
import tailwind from "bun-plugin-tailwind";

const root = new URL("..", import.meta.url).pathname;
const result = await Bun.build({
  entrypoints: [`${root}lane-leptos/css/rosegriffon.css`, `${root}lane-leptos/css/shenron.css`],
  outdir: `${root}dist/leptos-css`,
  naming: "[name].[ext]",
  minify: true,
  plugins: [tailwind],
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}
for (const out of result.outputs) console.log(`${out.path} ${out.size} bytes`);
