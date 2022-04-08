// @ts-check
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

import viteTsProto from "@lgn/vite-plugin-ts-proto";
import { loadAll } from "@lgn/web-client-config";

// import viteWasmPack from "@lgn/vite-plugin-wasm";

/** @type {"jsdom"} */
const testEnvironment = "jsdom";

/** @type {import("vite").Plugin[]} */
const plugins = [
  tsconfigPaths({
    extensions: [".ts", ".svelte"],
  }),
  viteTsProto({
    modules: [
      { name: "@lgn/proto-editor", glob: "*.proto" },
      { name: "@lgn/proto-runtime", glob: "*.proto" },
      { name: "@lgn/proto-streaming", glob: "*.proto" },
    ],
  }),
  // viteWasmPack({
  //   crates: [
  //     {
  //       path: "../../../npm-pkgs/simple-wasm-logger",
  //       packageName: "@lgn/simple-wasm-logger",
  //     },
  //   ],
  //   outDir: "frontend",
  //   quiet: true,
  // }),
];

if (process.env.VITEST) {
  plugins.push(svelte({ hot: false }));
}

export default defineConfig(() => {
  loadAll({
    VITE_ONLINE_AUTHENTICATION_OAUTH_ISSUER_URL:
      "online.authentication.issuer_url",
    VITE_ONLINE_AUTHENTICATION_OAUTH_CLIENT_ID:
      "online.authentication.client_id",
  });

  // https://vitejs.dev/config/
  return {
    // TODO: Drop this option when vite-tsconfig-paths
    // will work properly with SvelteKit
    resolve: {
      alias: {
        "@/resources": path.resolve("./tests/resources"),
        "@": path.resolve("./src"),
      },
    },
    plugins,
    test: {
      environment: testEnvironment,
      globals: true,
      setupFiles: "tests/setup.ts",
    },
  };
});
