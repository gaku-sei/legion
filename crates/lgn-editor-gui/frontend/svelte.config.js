// @ts-check

import preprocess from "svelte-preprocess";

export default {
  preprocess: preprocess({
    postcss: true,
    typescript: true,
  }),
};