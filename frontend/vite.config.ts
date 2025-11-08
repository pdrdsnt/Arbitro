import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  server: { port: 8001 },

  plugins: [vue()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
