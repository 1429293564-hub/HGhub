// Web 版兼容层：窗口控制 API 的空实现（浏览器里没有原生窗口）。
// 仅在 HGHUB_WEB=1 构建时经 vite alias 替换 @tauri-apps/api/window。

export function getCurrentWindow() {
  const noop = () => Promise.resolve();
  return {
    isMaximized: () => Promise.resolve(false),
    isFullscreen: () => Promise.resolve(false),
    onResized: () => Promise.resolve(() => {}),
    toggleMaximize: noop,
    minimize: noop,
    close: noop,
    startResizeDragging: (_direction: string) => Promise.resolve(),
  };
}
