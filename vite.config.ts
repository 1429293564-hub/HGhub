import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;
// HGHUB_WEB=1 → 构建「iOS 网页版」：tauri API 全部替换为同源 HTTP shim（src/web/），产物输出到 dist-web
const webMode = process.env.HGHUB_WEB === "1";

const { version } = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
) as { version: string };

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  resolve: webMode
    ? {
        alias: [
          {
            find: /^@tauri-apps\/api\/core$/,
            replacement: fileURLToPath(new URL("./src/web/core.ts", import.meta.url)),
          },
          {
            find: /^@tauri-apps\/api\/window$/,
            replacement: fileURLToPath(new URL("./src/web/window.ts", import.meta.url)),
          },
        ],
      }
    : undefined,
  // 版本号只维护 package.json（CI 会按 tag 改写它），界面里的版本由这里注入，避免写死后漂移
  define: {
    __APP_VERSION__: JSON.stringify(version),
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
    outDir: webMode ? "dist-web" : "dist",
    emptyOutDir: true,
  },
});
