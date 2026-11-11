//! HGhub Web 服务（axum）：复用客户端 Rust 核心模块，为浏览器提供同一套能力。
//!
//! 结构：
//! - 复用模块：net / session / lines / network_policy / logging 直接 `#[path]` 引入桌面端源码；
//!   api / media / playback_log 由 `server/gen.py` 生成（去 tauri 依赖的最小变换），改桌面端后重跑脚本同步。
//! - 路由：`POST /rpc`（Tauri 命令 → HTTP）、`GET /hls|/seg|/img`（原 hg:// 协议的 HTTP 代理端点）、
//!   静态托管前端 `dist-web/`。
//! - 登录门：除 /login 与 /healthz 外所有请求都要求会话 Cookie，未登录一律挡回登录页。
//!
//! 配置（环境变量）：
//! - `HGHUB_WEB_ADDR`      监听地址，默认 `127.0.0.1:8900`（上线验通后再改 `0.0.0.0:8900`）
//! - `HGHUB_DATA_DIR`      数据目录，默认 `./data`
//! - `HGHUB_WEB_PASSWORD`  登录密码；未设置时读取数据目录 `web-password.txt`，都没有则随机生成并写入该文件
//! - `HGHUB_DIST_DIR`      前端静态目录，默认 `./dist-web`

mod api;
#[path = "../../src-tauri/src/lines.rs"]
mod lines;
#[path = "../../src-tauri/src/logging.rs"]
mod logging;
mod media;
#[path = "../../src-tauri/src/net.rs"]
mod net;
#[path = "../../src-tauri/src/network_policy.rs"]
mod network_policy;
mod playback_log;
#[path = "../../src-tauri/src/session.rs"]
mod session;

use std::path::PathBuf;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::{json, Value};
use tower_http::services::{ServeDir, ServeFile};

use api::Api;

static DATA_DIR: OnceCell<PathBuf> = OnceCell::new();
static WEB_TOKEN: OnceCell<String> = OnceCell::new();

pub fn data_dir() -> PathBuf {
    DATA_DIR.get().cloned().unwrap_or_else(|| PathBuf::from("./data"))
}

fn cookie_name() -> &'static str {
    "hgsid"
}

fn read_cookie(req: &Request) -> Option<String> {
    let raw = req.headers().get(header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let pair = pair.trim();
        let cname = cookie_name();
        if let Some(v) = pair.strip_prefix(&format!("{cname}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn is_authed(req: &Request) -> bool {
    WEB_TOKEN
        .get()
        .zip(read_cookie(req))
        .is_some_and(|(t, c)| constant_eq(t, &c))
}

fn constant_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/* ───────────────────── 登录门 ───────────────────── */

async fn auth_middleware(req: Request, next: Next) -> Response {
    let path = req.uri().path();
    if path == "/login" || path == "/healthz" {
        return next.run(req).await;
    }
    if is_authed(&req) {
        return next.run(req).await;
    }
    // 页面请求重定向到登录页；数据接口返回 401（前端 shim 会跳登录页）
    let wants_html = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|a| a.contains("text/html"));
    if wants_html {
        Redirect::to("/login").into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

const LOGIN_PAGE: &str = r#"<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>HGhub · 登录</title>
<style>
  body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;
       background:#111418;color:#e8eaed;font-family:system-ui,-apple-system,"PingFang SC","Microsoft YaHei",sans-serif}
  .card{width:min(92vw,340px);padding:32px 28px;border-radius:16px;background:#1b2027;box-shadow:0 8px 32px rgba(0,0,0,.4)}
  h1{margin:0 0 6px;font-size:20px}
  p{margin:0 0 22px;color:#9aa3ad;font-size:13px}
  input{width:100%;box-sizing:border-box;padding:12px 14px;border-radius:10px;border:1px solid #333a44;
        background:#111418;color:#e8eaed;font-size:15px;outline:none}
  input:focus{border-color:#e8a33d}
  button{width:100%;margin-top:14px;padding:12px;border:0;border-radius:10px;background:#e8a33d;
         color:#111418;font-size:15px;font-weight:600;cursor:pointer}
  button:active{transform:translateY(1px)}
</style></head>
<body><form class="card" method="post" action="/login">
  <h1>HGhub</h1><p>私有部署 · 请输入访问密码</p>
  <input type="password" name="password" autocomplete="current-password" autofocus>
  <button type="submit">进入</button>
</form></body></html>"#;

async fn login_page() -> Html<&'static str> {
    Html(LOGIN_PAGE)
}

#[derive(serde::Deserialize)]
struct LoginForm {
    password: String,
}

async fn login_submit(axum::extract::Form(form): axum::extract::Form<LoginForm>) -> Response {
    let expected = std::env::var("HGHUB_WEB_PASSWORD").unwrap_or_default();
    let ok = if expected.is_empty() {
        // 密码来自 data/web-password.txt
        std::fs::read_to_string(data_dir().join("web-password.txt"))
            .map(|s| s.trim().to_string())
            .map(|pw| constant_eq(&pw, &form.password))
            .unwrap_or(false)
    } else {
        constant_eq(&expected, &form.password)
    };
    if ok {
        let mut resp = Redirect::to("/").into_response();
        let cname = cookie_name();
        let token = WEB_TOKEN.get().map(String::as_str).unwrap_or_default();
        if let Ok(v) = HeaderValue::from_str(&format!(
            "{cname}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000",
        )) {
            resp.headers_mut().insert(header::SET_COOKIE, v);
        }
        return resp;
    }
    (StatusCode::UNAUTHORIZED, Html(LOGIN_PAGE)).into_response()
}

/* ───────────────────── RPC 分发 ───────────────────── */

fn arg<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("参数 {key} 无效: {e}"))
}

fn v<T: Serialize>(r: Result<T, String>) -> Result<Value, String> {
    r.map(|x| serde_json::to_value(x).unwrap_or(Value::Null))
}

async fn rpc(State(api): State<std::sync::Arc<Api>>, req: Request) -> Response {
    let body = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => return rpc_err(format!("读取请求体失败: {e}")),
    };
    let parsed: Value = if body.is_empty() {
        Value::Null
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => return rpc_err(format!("请求体不是合法 JSON: {e}")),
        }
    };
    let cmd = parsed.get("cmd").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let args = parsed.get("args").cloned().unwrap_or(json!({}));

    let result = dispatch(&api, &cmd, &args).await;
    match result {
        Ok(v) => axum::Json(json!({ "result": v })).into_response(),
        Err(e) => rpc_err(e),
    }
}

/// 命令分发：签名与桌面端 invoke_handler 一一对应；`?` 在此可用。
async fn dispatch(api: &Api, cmd: &str, args: &Value) -> Result<Value, String> {
    match cmd {
        "app_boot" => api::app_boot(&api).await,
        "security_set" => v(api::security_set(arg(&args, "strict")?).await),
        "debug_log" => v(api::debug_log(arg(&args, "msg")?).await),
        "net_diag" => api::net_diag().await,
        "line_state" => api::line_state().await,
        "line_set" => v(api::line_set(&api, arg(&args, "url")?).await),
        "line_mirrors" => api::line_mirrors().await,
        "line_probe" => v(api::line_probe(arg(&args, "urls")?).await),
        "check_update" => api::check_update().await,
        "open_url" => v(api::open_url(arg(&args, "url")?).await),
        "set_orientation" => v(api::set_orientation(arg(&args, "landscape")?).await),
        "api_login" => v(api::api_login(&api, arg(&args, "username")?, arg(&args, "password")?).await),
        "api_register" => {
            v(api::api_register(
                &api,
                arg(&args, "username")?,
                arg(&args, "password")?,
                arg(&args, "email")?,
            ).await)
        }
        "api_logout" => api::api_logout(&api).await,
        "api_me" => api::api_me(&api).await,
        "api_feed" => v(api::api_feed(&api, arg(&args, "page")?, arg(&args, "size")?).await),
        "api_category" => {
            v(api::api_category(
                &api,
                arg(&args, "cate")?,
                arg(&args, "page")?,
                arg(&args, "size")?,
            ).await)
        }
        "api_rank" => api::api_rank(&api, arg(&args, "kind")?).await,
        "api_search" => {
            v(api::api_search(
                &api,
                arg(&args, "q")?,
                arg(&args, "page")?,
                arg(&args, "size")?,
            ).await)
        }
        "api_posts" => v(api::api_posts(&api, arg(&args, "page")?, arg(&args, "size")?).await),
        "api_post" => api::api_post(&api, arg(&args, "id")?).await,
        "api_video" => api::api_video(&api, arg(&args, "id")?).await,
        "api_play" => v(api::api_play(&api, arg(&args, "id")?, arg(&args, "ep")?).await),
        "api_comments" => {
            v(api::api_comments(
                &api,
                arg(&args, "id")?,
                arg(&args, "ep")?,
                arg(&args, "page")?,
                arg(&args, "size")?,
            ).await)
        }
        "api_comment_send" => {
            let reply_id: Option<i64> =
                serde_json::from_value(args.get("replyId").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("参数 replyId 无效: {e}"))?;
            api::api_comment_send(
                &api,
                arg(&args, "id")?,
                arg(&args, "ep")?,
                arg(&args, "text")?,
                reply_id,
            ).await
        }
        "api_like" => v(api::api_like(&api, arg(&args, "id")?, arg(&args, "ep")?, arg(&args, "on")?).await),
        "api_collect" => v(api::api_collect(&api, arg(&args, "id")?, arg(&args, "on")?).await),
        "api_follow" => v(api::api_follow(&api, arg(&args, "uid")?, arg(&args, "on")?).await),
        "cache_clear" => v(api::cache_clear().await),
        "api_post_like" => v(api::api_post_like(&api, arg(&args, "id")?, arg(&args, "on")?).await),
        "api_post_collect" => v(api::api_post_collect(&api, arg(&args, "id")?, arg(&args, "on")?).await),
        "api_post_comments" => {
            v(api::api_post_comments(&api, arg(&args, "id")?, arg(&args, "page")?, arg(&args, "size")?).await)
        }
        "playback_log_status" => v(playback_log::playback_log_status()),
        "playback_log_set" => v(playback_log::playback_log_set(arg(&args, "enabled")?)),
        "playback_log_write" => v(playback_log::playback_log_write(arg(&args, "entries")?)),
        "playback_log_read" => playback_log::playback_log_read().map(Value::String),
        _ => return Err(format!("未知命令: {cmd}")),
    }
}

fn rpc_err(msg: String) -> Response {
    (StatusCode::BAD_REQUEST, axum::Json(json!({ "error": msg }))).into_response()
}

/* ───────────────────── 媒体代理（原 hg:// 协议） ───────────────────── */

async fn proxy(req: Request) -> Response {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let resp = media::handle(&path, &query, "").await; // origin 传空 → 改写出的分片/key 为同源相对路径
    resp.map(Body::from)
}

/* ───────────────────── 启动 ───────────────────── */

fn resolve_password() -> String {
    if let Ok(pw) = std::env::var("HGHUB_WEB_PASSWORD") {
        if !pw.is_empty() {
            return pw;
        }
    }
    let file = data_dir().join("web-password.txt");
    if let Ok(existing) = std::fs::read_to_string(&file) {
        let pw = existing.trim().to_string();
        if !pw.is_empty() {
            return pw;
        }
    }
    use rand::RngCore;
    let mut buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut buf);
    let pw: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    let _ = std::fs::write(&file, &pw);
    println!("[hghub-web] 已生成随机登录密码，保存在 {}", file.display());
    pw
}

#[tokio::main]
async fn main() {
    let data_dir = std::env::var("HGHUB_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"));
    std::fs::create_dir_all(&data_dir).expect("无法创建数据目录");
    DATA_DIR.set(data_dir.clone()).ok();

    // 与桌面端 setup 等价：恢复线路/安全策略/播放日志/会话
    net::bind_base(&data_dir);
    playback_log::bind(&data_dir);
    network_policy::bind_security(&data_dir);

    let api = std::sync::Arc::new(Api::new());
    api.session.bind(data_dir.clone());

    // 会话令牌：进程启动时随机生成（重启后旧 Cookie 失效，可接受）
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let token: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    WEB_TOKEN.set(token).ok();

    let _password = resolve_password();

    let dist = std::env::var("HGHUB_DIST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("dist-web"));
    println!("[hghub-web] 前端目录: {}", dist.display());
    println!("[hghub-web] 数据目录: {}", data_dir.display());

    let static_dir = ServeDir::new(&dist)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(dist.join("index.html")));

    let addr: std::net::SocketAddr = std::env::var("HGHUB_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8900".to_string())
        .parse()
        .expect("HGHUB_WEB_ADDR 无效");

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/login", get(login_page).post(login_submit))
        .route("/rpc", post(rpc))
        .route("/hls", get(proxy))
        .route("/seg", get(proxy))
        .route("/img", get(proxy))
        .fallback_service(static_dir)
        .layer(middleware::from_fn(auth_middleware))
        .with_state(api);

    println!("[hghub-web] 监听 {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("端口绑定失败");
    axum::serve(listener, app).await.expect("服务异常退出");
}
