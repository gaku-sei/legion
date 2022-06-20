// @ts-check
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

import viteApiCodegen from "@lgn/vite-plugin-api-codegen";

process.env.VITE_CONSOLE_LOG_LEVEL = "debug";

// https://vitejs.dev/config/
export default defineConfig(() => {
  return {
    plugins: [
      tsconfigPaths({
        extensions: [".ts", ".svelte"],
      }),
      // The `!!` trick is necessary here or the whole expression
      // will return `undefined` which make hot `true`...
      // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
      svelte({ hot: !!process.env.DEV && !process.env.VITEST }),
      viteApiCodegen({
        aliasMappings: {
          "../../crates/lgn-governance/apis/space.yaml": "Space",
          "../../crates/lgn-governance/apis/workspace.yaml": "Workspace",
        },
        apiOptions: [
          {
            path: "../../crates/lgn-streamer/apis",
            names: ["streaming"],
            filename: "streaming",
          },
          {
            path: "../../crates/lgn-log/apis",
            names: ["log"],
            filename: "log",
          },
        ],
      }),
    ],
    test: {
      environment: "jsdom",
      globals: true,
      setupFiles: "tests/setup.ts",
    },
  };
});
