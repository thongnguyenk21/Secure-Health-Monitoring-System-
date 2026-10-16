import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The backend listens on :8080 in development (override with BACKEND_URL); the
// proxy keeps the app and the API on one origin so the WebSocket URL can be
// derived from `location`.
const BACKEND_URL = process.env.BACKEND_URL ?? "http://localhost:8080";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": { target: BACKEND_URL, changeOrigin: true, ws: true },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
