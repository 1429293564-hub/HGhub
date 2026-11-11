#!/usr/bin/env python3
"""从 src-tauri 生成 server 侧复用模块。

server 不依赖 tauri 运行时，但业务核心（api/media/playback_log）与桌面端共用源码。
本脚本做最小机械变换：
  - api.rs        → server/src/api.rs        State<'_, Api> 改 &Api，AppHandle 三命令改服务器实现
  - media.rs      → server/src/media.rs      tauri::http → http
  - playback_log.rs → server/src/playback_log.rs  去掉 #[tauri::command] 属性（函数体本就是纯函数）
运行：python server/gen.py（在仓库根目录执行）
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src-tauri" / "src"
OUT = ROOT / "server" / "src"
OUT.mkdir(parents=True, exist_ok=True)

def strip_commands(text: str) -> str:
    return text.replace("#[tauri::command]\n", "")

# ── media.rs：仅替换 http 类型导入 ─────────────────────────────
media = (SRC / "media.rs").read_text(encoding="utf-8")
assert "use tauri::http::{Response, StatusCode};" in media
media = media.replace(
    "use tauri::http::{Response, StatusCode};",
    "use http::{Response, StatusCode};",
)
(OUT / "media.rs").write_text(media, encoding="utf-8")
print("media.rs ok")

# ── playback_log.rs：去掉 tauri 命令属性，函数体原样 ──────────
plog = strip_commands((SRC / "playback_log.rs").read_text(encoding="utf-8"))
(OUT / "playback_log.rs").write_text(plog, encoding="utf-8")
print("playback_log.rs ok")

# ── api.rs：State→&Api，AppHandle 命令改服务器实现，去测试 ────
api = strip_commands((SRC / "api.rs").read_text(encoding="utf-8"))
api = api.replace("use tauri::State;\nuse tauri_plugin_opener::OpenerExt;\n", "")
api = api.replace("state: State<'_, Api>", "state: &Api")
# 截掉末尾 tests 模块
api = api[: api.index("#[cfg(test)]")]

# debug_log：写入服务器数据目录
api = re.sub(
    r"pub async fn debug_log\(.*?\n\}\n",
    '''pub async fn debug_log(msg: String) -> Result<(), String> {
    let dir = crate::data_dir();
    crate::logging::append_debug(&dir, &msg)
}
''',
    api, count=1, flags=re.S,
)
# open_url：服务器侧只校验目标地址，不真正打开浏览器
api = re.sub(
    r"pub async fn open_url\(.*?\n\}\n",
    '''pub async fn open_url(url: String) -> Result<(), String> {
    let _target = crate::network_policy::parse_target(&url)?;
    Ok(())
}
''',
    api, count=1, flags=re.S,
)
# set_orientation：无窗口系统，空实现
api = re.sub(
    r"(?s)pub async fn set_orientation\(.*?\n\}\n",
    '''pub async fn set_orientation(landscape: bool) -> Result<(), String> {
    let _ = landscape;
    Ok(())
}
''',
    api, count=1,
)
# request_ios_orientation 仅 iOS 用，服务器裁掉（非贪婪，只删到该函数结尾）
api = re.sub(r"#\[cfg\(target_os = \"ios\"\)\]\nfn request_ios_orientation.*?\n\}\n", "", api)

(OUT / "api.rs").write_text(api, encoding="utf-8")
print("api.rs ok")
