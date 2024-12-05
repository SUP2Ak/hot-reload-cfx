import { defineConfig } from 'vite'
import { resolve } from 'path'

export default defineConfig({
  build: {
    lib: {
      entry: {
        // client: resolve(__dirname, 'src/client/client.ts'),
        server: resolve(__dirname, 'src/server/main.ts'),
      },
      formats: ['cjs'],
      fileName: (format, entryName) => `${entryName}.js`
    },
    rollupOptions: {
      external: ['@citizenfx/client', '@citizenfx/server'],
      output: {
        // Désactiver le hachage des noms de fichiers
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