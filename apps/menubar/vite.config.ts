import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [svelte()],

  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
  },

  build: {
    // WKWebView only, and minimumSystemVersion is macOS 13
    target: "safari16",
  },
});
