// Web 版兼容层：把 Tauri 的 invoke 桥接到同源 HTTP（服务端 POST /rpc）。
// 仅在 HGHUB_WEB=1 构建时经 vite alias 替换 @tauri-apps/api/core。
type CmdArgs = Record<string, unknown> | undefined;

export async function invoke<T = unknown>(cmd: string, args?: CmdArgs): Promise<T> {
  const res = await fetch("/rpc", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ cmd, args: args ?? {} }),
  });
  if (res.status === 401) {
    // 登录门：会话过期/未登录 → 回登录页
    window.location.href = "/login";
    throw new Error("未登录");
  }
  const data = (await res.json().catch(() => null)) as
    | { result?: T; error?: string }
    | null;
  if (!res.ok) {
    throw new Error(data?.error ?? `HTTP ${res.status}`);
  }
  if (data && typeof data.error === "string" && data.error) {
    throw new Error(data.error);
  }
  return (data?.result ?? null) as T;
}

export function addPluginListener(
  _plugin: string,
  _event: string,
  _cb: (...args: unknown[]) => void,
): Promise<{ unregister: () => Promise<void> }> {
  // Web 版无原生插件事件（安卓返回键等），空实现
  return Promise.resolve({ unregister: () => Promise.resolve() });
}
