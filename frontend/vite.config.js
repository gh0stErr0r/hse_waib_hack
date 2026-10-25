import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      "/api/v1": "http://localhost:4000",
      "/api/auth": "http://localhost:8080",
      "/ws": { target: "ws://localhost:4000", ws: true },
    },
  },
});
