/**
 * Build script: bundles bin/cli.mjs into a standalone Node.js script
 * using esbuild (bundled with Vite). The output goes to bin/cli-bundle.mjs
 * which will be bundled as a Tauri resource.
 *
 * Run: node bin/build-cli.js
 */

import { build } from 'esbuild';

await build({
  entryPoints: ['bin/cli.mjs'],
  bundle: true,
  platform: 'node',
  target: 'node22',
  format: 'esm',
  outfile: 'bin/cli-bundle.mjs',
  external: [
    // Native Node.js modules that shouldn't be bundled
    'fsevents',
    // jsdom has native canvas dependencies; exclude those from bundling
    'canvas',
  ],
  minify: false,
  sourcemap: false,
});

console.log('CLI bundle built: bin/cli-bundle.mjs');
