import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    pool: 'forks',
    execArgv: ['--experimental-wasm-jspi'],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/.fox-tests/**',
      '**/.fox-benchs/**'
    ]
  }
})
