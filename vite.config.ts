import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

/**
 * `tauri dev` exports this when targeting a device on the local network.
 *
 * Read through `globalThis` rather than importing `node:process`, so the config
 * type-checks without pulling in `@types/node` just for one environment lookup.
 */
const devHost = (
  globalThis as { process?: { env?: Record<string, string | undefined> } }
).process?.env?.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [vue()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: devHost || false,
    hmr: devHost
      ? {
          protocol: "ws",
          host: devHost,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
