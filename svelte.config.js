import adapter from '@sveltejs/adapter-static';
import 'dotenv/config';
import { sveltePreprocess } from 'svelte-preprocess';

const isTauri = process.env.TAURI_ENV_ARCH !== undefined;

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: [sveltePreprocess({})],
  kit: {
    alias: {
      '$/*': './src/lib/*'
    },
    paths: {
      // Tauri requires relative paths (empty base), web deploy may use MERMAID_BASE_PATH
      base: isTauri ? '' : (process.env.MERMAID_BASE_PATH ?? '')
    },
    adapter: adapter({
      pages: 'docs',
      fallback: '404.html'
    })
  }
};

export default config;
