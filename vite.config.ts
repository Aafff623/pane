import { defineConfig } from "vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  // Build stamp for the footer, e.g. "0707.1432" (MMDD.HHmm).
  define: {
    __BUILD_STAMP__: JSON.stringify(
      new Date()
        .toISOString()
        .slice(5, 16)
        .replace("-", "")
        .replace("T", ".")
        .replace(":", ""),
    ),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // Only the app is a dependency entry. Local previews and the separate site
  // must not join the desktop app's dependency scan.
  optimizeDeps: { entries: ["index.html"] },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Cargo outputs and local previews can contain thousands of files and
      // trigger unrelated reloads. Watch only the desktop frontend's inputs.
      ignored: [
        "**/src-tauri/**",
        "**/parse-tests/**",
        "**/temp/**",
        "**/site/**",
        "**/.zcode/**",
      ],
    },
  },
}));
