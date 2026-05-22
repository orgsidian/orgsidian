import { defineConfig } from "vite";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react-swc";
import { lingui } from "@lingui/vite-plugin";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    // MUST come before tailwindcss() and react() per TanStack docs.
    tanstackRouter({ target: "react", autoCodeSplitting: true }),
    tailwindcss(),
    react({ plugins: [["@lingui/swc-plugin", {}]] }),
    lingui(),
  ],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: false,
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
