import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
import { cpSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
const host = process.env.TAURI_DEV_HOST;

// Copies launcher/backgrounds/ into public/assets/backgrounds/ so Vite serves it. Runs on every dev or build start.
const syncBackgrounds = {
  name: "sync-backgrounds",
  buildStart() {
    const src = fileURLToPath(new URL("./backgrounds", import.meta.url));
    if (existsSync(src)) {
      cpSync(src, fileURLToPath(new URL("./public/assets/backgrounds", import.meta.url)), { recursive: true });
    }
  },
};

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss(), syncBackgrounds],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
