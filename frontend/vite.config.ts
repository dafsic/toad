import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
    plugins: [react()],
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
    // 开发时代理 API 请求至 Rust 后端，避免跨域
    server: {
        proxy: {
            '/api': 'http://localhost:3000',
        },
    },
    build: {
        outDir: 'dist',
        emptyOutDir: true,
    },
})
