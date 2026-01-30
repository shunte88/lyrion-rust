import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { VitePWA } from 'vite-plugin-pwa';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['*.png', '*.svg'],
      manifest: false, // Use public/manifest.json instead
      workbox: {
        // Cache API responses
        runtimeCaching: [
          {
            urlPattern: /^http:\/\/localhost:9000\/api\/.*/i,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'api-cache',
              expiration: {
                maxEntries: 100,
                maxAgeSeconds: 60 * 60 * 24, // 24 hours
              },
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
          {
            urlPattern: /^http:\/\/localhost:9000\/api\/v1\/cover\/.*/i,
            handler: 'CacheFirst',
            options: {
              cacheName: 'cover-art-cache',
              expiration: {
                maxEntries: 500,
                maxAgeSeconds: 60 * 60 * 24 * 30, // 30 days
              },
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
        ],
        // Offline fallback
        navigateFallback: '/index.html',
        navigateFallbackDenylist: [/^\/api/, /^\/ws/, /^\/stream/],
      },
      devOptions: {
        enabled: false, // Disable in dev to avoid issues
      },
    }),
  ],
  server: {
    port: 3000,
    proxy: {
      // Proxy API requests to Rust backend
      '/api': {
        target: 'http://localhost:9000',
        changeOrigin: true,
      },
      '/jsonrpc.js': {
        target: 'http://localhost:9000',
        changeOrigin: true,
      },
      '/stream': {
        target: 'http://localhost:9000',
        changeOrigin: true,
      },
      '/ws': {
        target: 'http://localhost:9000',
        changeOrigin: true,
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
  },
});
