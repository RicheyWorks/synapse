import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

// Tauri expects the dev server on a fixed port and the built assets in
// `dist` (referenced as `../dist` from src-tauri/tauri.conf.json).
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
  },
})
