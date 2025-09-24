import { defineConfig } from "vite";
export default defineConfig({
  server: { port: 8001 },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  }
});

