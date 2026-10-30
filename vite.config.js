import { defineConfig } from 'vite';
export default defineConfig({
  clearScreen: false,
  server: { strictPort: true, watch: { ignored: ['**/src-tauri/**', '**/.local-state/**'] } },
  build: { target: 'es2022' },
});
