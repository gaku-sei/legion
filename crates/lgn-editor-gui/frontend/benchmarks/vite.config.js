// @ts-check

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";
import viteTsProto from "@lgn/vite-plugin-ts-proto";
import path from "path";
import { fileURLToPath } from "url";

// https://vitejs.dev/config/
export default defineConfig({
  build: {
    lib: {
      entry: path.resolve(
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        path.dirname(fileURLToPath(import.meta.url)),
        "index.ts"
      ),
      name: "benchmarks",
      fileName: () => "benchmarks/index.js",
    },
  },
  plugins: [
    tsconfigPaths({
      extensions: [".ts", ".svelte"],
    }),
    svelte(),
    viteTsProto({
      modules: [
        { name: "@lgn/proto-editor", glob: "*.proto" },
        { name: "@lgn/proto-streaming", glob: "*.proto" },
      ],
    }),
  ],
});
