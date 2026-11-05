import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ["browser"]
  },
  server: {
    fs: {
      allow: ["../.."]
    }
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts", "../../tests/e2e/**/*.spec.ts"],
    restoreMocks: true
  }
});
