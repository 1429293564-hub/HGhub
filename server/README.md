# HGhub Web（iOS 网页版服务）

复用客户端 Rust 核心（`src-tauri/src`）的独立 HTTP 服务，把原来只在 App 内可用的
数据请求、解密、HLS 视频流转发搬到了服务端，浏览器（含 iPhone Safari）直接可用。

## 结构

| 模块 | 来源 | 说明 |
| --- | --- | --- |
| `server/src/net.rs` 等 5 个 | `#[path]` 直引 `src-tauri/src` | 网络 / 会话 / 线路 / 安全策略 / 日志，零改动 |
| `server/src/api.rs` | `server/gen.py` 生成 | Tauri 命令 → 纯函数（`State<Api>` → `&Api`，AppHandle 命令改服务器实现） |
| `server/src/media.rs` | `server/gen.py` 生成 | 仅把 `tauri::http` 换成 `http`；HLS 改写、封面解密逻辑原样 |
| `server/src/playback_log.rs` | `server/gen.py` 生成 | 去掉 `#[tauri::command]` 属性，函数体原样 |

改了桌面端核心代码后，在仓库根目录跑 `python server/gen.py` 同步。

## 路由

- `GET  /login` / `POST /login`：密码登录门（未登录一切不可见）
- `POST /rpc`：`{cmd, args}` 分发到与桌面端相同的命令集（app_boot/api_feed/api_play/…）
- `GET  /hls?u=…`、`/seg?u=…`、`/img?u=…`：原 `hg://` 自定义协议的 HTTP 代理端点
- 其余路径：托管 `dist-web/`（前端 web 构建产物，SPA fallback 到 index.html）
- `GET /healthz`：健康检查（免登录，供 systemd/监控用）

## 前端适配

- `src/api.ts`：`LOCAL` 在无 Tauri 运行时时为空串，`/hls`、`/img`、`/seg` 走同源相对路径
- `src/web/core.ts` / `src/web/window.ts`：`invoke` → `fetch("/rpc")` 的 shim，`HGHUB_WEB=1`
  构建时经 vite alias 替换 `@tauri-apps/api/*`

## 配置（环境变量，见 deploy/hghub-web.service + .env）

| 变量 | 默认 | 说明 |
| --- | --- | --- |
| `HGHUB_WEB_ADDR` | `127.0.0.1:8900` | 监听地址。**先绑本机验通，再改 `0.0.0.0:8900` 对外** |
| `HGHUB_DATA_DIR` | `./data` | 线路选择、会话、播放日志存放目录 |
| `HGHUB_WEB_PASSWORD` | 无 | 登录密码；不设则用 `data/web-password.txt`（首次启动自动生成随机密码） |
| `HGHUB_DIST_DIR` | `./dist-web` | 前端静态目录 |

## 部署顺序（别反）

1. 构建：GitHub Actions `Build Web Server`（workflow_dispatch）→ 下载 `webserver-linux` 产物
2. 上传服务器 → `sudo bash deploy.sh`（此时监听 `127.0.0.1:8900`）
3. 服务器本地验通：`curl -s localhost:8900/healthz`；未登录 302 → /login；登录后 feed 是真实数据
4. 验通后才改 `.env` 里 `HGHUB_WEB_ADDR=0.0.0.0:8900` 并 `systemctl restart hghub-web`
5. 云控制台安全组放行 TCP 8900
6. 外网验真：未登录被挡回登录页；登录后首页 feed 真实数据；任选一部剧详情可开、
   HLS 清单与分片经代理返回 200、封面能加载
7. iPhone Safari 打开 → 分享 → 添加到主屏幕

## 许可

同上游：PolyForm Noncommercial，仅限个人非商业使用。
