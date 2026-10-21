import { defineConfig } from 'vitest/config'
import tailwindcss from '@tailwindcss/vite'
import { tanstackRouter } from '@tanstack/router-plugin/vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [tanstackRouter({ target: 'react', quoteStyle: 'single', semicolons: false, addExtensions: true }), react(), tailwindcss()],
  resolve: { tsconfigPaths: true },
  worker: { format: 'es' },
  test: { include: ['tests/**/*.test.ts'], setupFiles: ['tests/setup.ts'] },
})
