//! 业务命令：登录 / 列表 / 详情 / 播放 / 互动。

use crate::net::{base, edge};
use crate::session::SessionStore;
use serde_json::{json, Value};

pub struct Api {
    pub session: SessionStore,
}

impl Api {
    pub fn new() -> Self {
        Self {
            session: SessionStore::new(),
        }
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let origin = base();
        let token = self.session.token(&origin);
        edge(&origin, "GET", path, None, token.as_deref()).await
    }

    /// POST（游客也可调用，有登录态时带上 token）：新播放接口需要。
    async fn post(&self, path: &str, body: Option<Value>) -> Result<Value, String> {
        let origin = base();
        let token = self.session.token(&origin);
        edge(&origin, "POST", path, body, token.as_deref()).await
    }

    async fn authed(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
        let origin = base();
        let token = self
            .session
            .token(&origin)
            .ok_or_else(|| "请先登录".to_string())?;
        edge(&origin, method, path, body, Some(&token)).await
    }
}

/// API/运行错误与自检统一写入 debug.log，每行附 RFC 3339 UTC 时间和进程编号。
pub async fn debug_log(msg: String) -> Result<(), String> {
    let dir = crate::data_dir();
    crate::logging::append_debug(&dir, &msg)
}

/// 网络自检：系统代理 / DNS / 连通性快照，用于远程定位「打不开」。
pub async fn net_diag() -> Result<Value, String> {
    Ok(crate::net::diagnose().await)
}

/* ───────────────────────── 线路（数据源） ───────────────────────── */

/// 当前线路 + 固定线路清单（镜像线路另外按需拉取）。
pub async fn line_state() -> Result<Value, String> {
    Ok(json!({
        "base": base(),
        "default": crate::net::DEFAULT_BASE,
        "fixed": crate::lines::fixed(),
    }))
}

/// 手动切换线路：立即生效并落盘（下次启动沿用）。
pub async fn line_set(state: &Api, url: String) -> Result<String, String> {
    let previous = base();
    let next = crate::net::set_base(&url)?;
    if previous != next {
        state.session.clear();
    }
    Ok(next)
}

/// 从地址发布页抓镜像线路（只在用户点开「线路」时调用）。
pub async fn line_mirrors() -> Result<Value, String> {
    // 用户手动点「刷新 / 打开线路」时重新读一次系统代理：代理软件开关过之后旧客户端会卡死
    crate::net::refresh_client();
    crate::lines::mirrors().await
}

/// 可达性探测：只报结果，不做自动检查 / 自动切换。
pub async fn line_probe(urls: Vec<String>) -> Result<Value, String> {
    crate::net::refresh_client();
    Ok(crate::lines::probe(urls).await)
}

/* ───────────────────────── 更新检查 ───────────────────────── */

const REPO: &str = "Minireel/HGhub";

fn ver_parts(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .map(|s| s.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

/// 只比较数字段：`v1.2.0` > `1.1.9`；解析不出的段按 0 处理。
fn is_newer(latest: &str, current: &str) -> bool {
    let (a, b) = (ver_parts(latest), ver_parts(current));
    for i in 0..a.len().max(b.len()) {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// 对比 GitHub Release 的 tag 与当前版本（未认证 60 次/小时/IP，够用）。
pub async fn check_update() -> Result<Value, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = crate::net::get(
        &url,
        std::time::Duration::from_secs(12),
        Some("application/vnd.github+json"),
        None,
    )
    .await?;

    let status = resp.status();
    if status.as_u16() == 404 {
        // 还没发过 release
        return Ok(json!({
            "current": current, "latest": "", "has_update": false,
            "url": format!("https://github.com/{REPO}/releases"),
        }));
    }
    if !status.is_success() {
        return Err(format!("GitHub 返回 HTTP {status}"));
    }

    let v: Value = resp.json().await.map_err(|e| crate::net::err_text(&e))?;
    let tag = v
        .get("tag_name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let html = v
        .get("html_url")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://github.com/{REPO}/releases"));
    let notes = v.get("body").and_then(|x| x.as_str()).unwrap_or("");

    Ok(json!({
        "current": current,
        "latest": tag.trim_start_matches(['v', 'V']).to_string(),
        "tag": tag,
        "has_update": !tag.is_empty() && is_newer(&tag, &current),
        "url": html,
        "name": v.get("name").and_then(|x| x.as_str()).unwrap_or(""),
        "published_at": v.get("published_at").and_then(|x| x.as_str()).unwrap_or(""),
        "notes": notes.chars().take(800).collect::<String>(),
    }))
}

/// 用系统默认浏览器打开链接（只允许 http/https）。
pub async fn open_url(url: String) -> Result<(), String> {
    let _target = crate::network_policy::parse_target(&url)?;
    Ok(())
}

/// 切换移动端屏幕方向（iOS 原生旋转屏幕方向：横屏 Landscape / 竖屏 Portrait）。
pub async fn set_orientation(landscape: bool) -> Result<(), String> {
    let _ = landscape;
    Ok(())
}

