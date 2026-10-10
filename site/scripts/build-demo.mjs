import { createRequire } from 'node:module';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';

const site = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const source = resolve(process.argv[2] || resolve(site, '..'));
const require = createRequire(resolve(source, 'package.json'));
let viteEntry;
try { viteEntry = require.resolve('vite'); } catch { viteEntry = require.resolve('vite', { paths: [resolve('D:/code/pane/node_modules')] }); }
const { build } = await import(pathToFileURL(viteEntry).href);
const pkg = JSON.parse(await readFile(resolve(source, 'package.json'), 'utf8'));
const outDir = resolve(site, 'public/demo');
await mkdir(outDir, { recursive: true });
await build({
  configFile: false,
  root: source,
  base: '/demo/',
  publicDir: false,
  define: { __BUILD_STAMP__: JSON.stringify('website'), __SOURCE_VERSION__: JSON.stringify(pkg.version) },
  resolve: { alias: [
    // bridge.ts must read the SAME providerCatalog copy as the app — a bare
    // relative import resolves next to this worktree's own (stale) src/.
    { find: /^pane-source\/providerCatalog$/, replacement: resolve(source, 'src/providerCatalog.ts') },
    ...['core', 'event', 'app', 'webviewWindow'].map(name => ({
      find: `@tauri-apps/api/${name}`, replacement: resolve(site, 'demo/bridge.ts'),
    })),
  ] },
  plugins: [{ name: 'pane-website-demo', transformIndexHtml: {
    order: 'pre', handler(html) {
      // An embedded demo has no window of its own: whenever the guest
      // document loads hidden (a background tab, or a host app whose
      // browser pane is not focused — ZCode's in-app browser keeps its
      // guest hidden even while the host page is visible), the browser
      // suspends that document's requestAnimationFrame. The panel paints
      // on rAF, so the first paint — and every coalesced repaint behind
      // scheduleRender's renderFrame flag — would never run and the cards
      // would stay on the static skeleton while data refreshes underneath.
      // This inline shim runs before the app module and routes rAF onto
      // setTimeout for hidden guests only; a visible guest keeps the
      // native rAF. Paired with the document.hidden rewrite in the
      // always-visible plugin below, the demo keeps painting everywhere
      // it is embedded.
      // rAF is rerouted unconditionally, before the app module runs. The guest
      // may be embedded where render frames are suspended (a background tab, or
      // a host whose browser pane is not painting) and the panel coalesces its
      // paints behind a single renderFrame rAF handle — one missed native frame
      // wedges every later repaint, including the manual refresh. setTimeout
      // keeps it painting everywhere; a visible browser just gets 16ms ticks
      // instead of vsync-aligned ones. The DOMContentLoaded backstop then only
      // nudges a refresh if the static skeleton is somehow still on screen.
      const shim = '<script>(function(){window.requestAnimationFrame=function(cb){return window.setTimeout(function(){cb(window.performance.now())},16)};window.cancelAnimationFrame=function(id){window.clearTimeout(id)};document.addEventListener("DOMContentLoaded",function(){setTimeout(function(){if(document.querySelector(".skeleton-card")){var b=document.querySelector("#refresh");if(b)b.click()}},900)})})()</script>';
      return html.replace('</head>', '<link rel="stylesheet" href="/fonts/fonts.css"><link rel="stylesheet" href="/demo-frame.css?v=7">' + shim + '</head>');
    },
  } }, {
    // The desktop app parks its repaints while its window is hidden:
    // renderIfVisible / scheduleRender / tickCountdowns all read
    // document.hidden and defer to a pendingRender flag. The website demo
    // is an iframe with no window of its own, so a visitor's background
    // tab (or a minimized host app) would freeze the panel on its static
    // skeleton — data keeps refreshing (the footer timestamp moves) while
    // the cards never paint. The demo build rewrites those reads to
    // "always visible" so the panel keeps painting everywhere it is
    // embedded; the refresh timer and reveal animation simply stay live,
    // which is what an embedded demo wants anyway.
    name: 'pane-website-demo-always-visible',
    renderChunk(code) {
      return code.includes('document.hidden')
        ? code.replaceAll('document.hidden', 'false')
        : code;
    },
  }],
  build: { outDir, emptyOutDir: false, rollupOptions: { input: resolve(source, 'index.html') } },
});
// Vite preserves some CRLF lines from the Windows source HTML while injecting
// LF markup. Normalize only the generated demo, leaving the shared app alone.
const demoHtmlPath = resolve(outDir, 'index.html');
await writeFile(demoHtmlPath, (await readFile(demoHtmlPath, 'utf8')).replaceAll('\r\n', '\n'));
const files = ['index.html', 'src/main.ts', 'src/styles.css', 'src/i18n.ts', 'src/providerCatalog.ts'];
const hashes = {};
// Hash CRLF-normalized content so a Windows checkout (autocrlf) records the
// same provenance as the LF checkout CI verifies against.
for (const file of files) hashes[file] = createHash('sha256').update((await readFile(resolve(source, file), 'utf8')).replaceAll('\r\n', '\n')).digest('hex');
await writeFile(resolve(outDir, 'source.json'), JSON.stringify({ version: pkg.version, files: hashes }, null, 2));
console.log(`Pane ${pkg.version} real frontend bundled to ${outDir}`);
