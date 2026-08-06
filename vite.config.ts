import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

// 前端只負責畫面與事件，patch 引擎在 Rust（src-tauri），因此不需要 node polyfill，
// 相依比 RICH2_EDITOR 乾淨得多。
export default defineConfig({
    plugins: [
        tailwindcss(),
    ],
    // Tauri 會盯著 1420 以外的埠，這裡沿用 Vite 預設 5173（tauri.conf.json 的 devUrl 要一致）
    clearScreen: false,
    server: {
        strictPort: true,
    },
});
