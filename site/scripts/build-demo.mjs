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
      return html.replace('</head>', '<link rel="stylesheet" href="/fonts/fonts.css"><link rel="stylesheet" href="/demo-frame.css?v=7"></head>');
    },
  } }],
  build: { outDir, emptyOutDir: false, rollupOptions: { input: resolve(source, 'index.html') } },
});
const files = ['index.html', 'src/main.ts', 'src/styles.css', 'src/i18n.ts', 'src/providerCatalog.ts'];
const hashes = {};
for (const file of files) hashes[file] = createHash('sha256').update(await readFile(resolve(source, file))).digest('hex');
await writeFile(resolve(outDir, 'source.json'), JSON.stringify({ version: pkg.version, files: hashes }, null, 2));
console.log(`Pane ${pkg.version} real frontend bundled to ${outDir}`);
