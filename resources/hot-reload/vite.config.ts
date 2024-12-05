import { defineConfig } from 'vite'
import { resolve } from 'path'

export default defineConfig({
  build: {
    lib: {
      entry: {
        server: resolve(__dirname, 'src/server/main.ts'),
      },
      formats: ['cjs'],
      fileName: (_: string, entryName: string) => `${entryName}.js`
    },
    rollupOptions: {
      external: ['@citizenfx/server', 'ws'],
      output: {
        entryFileNames: '[name].js',
        chunkFileNames: '[name].js',
        assetFileNames: '[name].[ext]'
      }
    },
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: true,
  },
})