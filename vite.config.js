import { defineConfig } from "vite-plus";
export default defineConfig({
  fmt: {},
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  optimizeDeps: { exclude: ['linen-ui'] },
  clearScreen: false,
  server: { strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/.local-state/**"] } },
  build: { target: "es2022" },
});
