import { defineConfig } from "vitest/config";
import path from "node:path";

export default defineConfig({
  resolve: {
    alias: {
      $lib: path.resolve("./src/lib"),
    },
  },
  test: {
    environment: "node",
    include: ["src/lib/vnext/**/*.test.ts"],
    reporters: ["default"],
  },
});
