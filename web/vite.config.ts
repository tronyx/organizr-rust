import { fileURLToPath } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  // Relative addresses, so the interface works at the root of a host and in a sub-folder.
  base: './',
  resolve: { alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) } },
  plugins: [tailwindcss(), svelte(), svelteTesting({ autoCleanup: false })],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    // The polyfill is an inline script, which the Content-Security-Policy does not allow.
    modulePreload: { polyfill: false },
    // A small font file would become a `data:` address, which `font-src 'self'` blocks:
    // fonts of every kind always stay files.
    assetsInlineLimit: (file) => (/\.(woff2?|ttf|otf|eot)$/.test(file) ? false : undefined),
  },
  server: {
    // `npm run dev` serves the interface and sends API requests to a server on this machine.
    proxy: { '/api': 'http://127.0.0.1:8080' },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    setupFiles: ['src/test-setup.ts'],
    restoreMocks: true,
    unstubGlobals: true,
  },
});
