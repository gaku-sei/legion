import mkdirp from "mkdirp";
import path from "path";
import { generate } from "@lgn/api-codegen";

export type Config = {
  /** Root path of the apis definitions files (.yaml) */
  path: string;
  /** An array containing the api to generate the client(s) for */
  apiNames: string[];
};

/**
 * Automatically generates TypeScript code from OpenAPI files before Vite build
 */
export default function vitePluginApiCodegen({
  path: apisPath,
  apiNames,
}: Config) {
  return {
    name: "@lgn/vite-plugin-api-codegen",
    async buildStart() {
      const outDir = path.resolve(process.cwd(), "node_modules", "@lgn/apis");

      await mkdirp(outDir);

      generate({ apiNames, path: apisPath, outDir });
    },
  };
}
