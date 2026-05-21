import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

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
      // 3. tell Vite to ignore watching the Rust shell app and workspace build outputs.
      //    BMAD markdown churn (epics.md, sprint-status.yaml, etc.) is also ignored
      //    to avoid spurious Vite reloads while planning artifacts evolve.
      ignored: [
        "**/crates/orgsidian-shell-app/**",
        "**/target/**",
        "**/_bmad-output/**",
        "**/_bmad/**",
      ],
    },
  },
}));
