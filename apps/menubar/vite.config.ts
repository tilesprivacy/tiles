import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
  // the chat ui owns the root of the bundle, this sits beside it
  base: "/panel/",

  plugins: [svelte()],

  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
  },

  build: {
    outDir: "dist/panel",
    // WKWebView only, and minimumSystemVersion is macOS 13
    target: "safari16",
  },
});
