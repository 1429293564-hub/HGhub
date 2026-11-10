# DEPLOY.md — 给 AI 助手的执行说明（已实测跑通）

你是一个带浏览器和终端能力的智能体执行者。用户让你“照着这个仓库把安装包打包出来”时，按本文执行。目标产物：Windows 安装包（.exe）、安卓 APK（arm64 为主）、macOS dmg（附带）。全程在用户的 GitHub 账号下操作，不需要用户本地安装任何开发工具。

参考实现：Muse（2026-11-10 实测全流程跑通）；同流程适用于任何能操作浏览器、能运行终端命令的智能体。

## 执行者契约

1. 只做本文列出的动作：fork、启用 Actions、生成签名密钥、填 Secret、触发并监控流水线、下载产物、交付用户。不点 star、不关注、不做任何额外宣传动作（原作者明确希望低调）。
2. 需要登录时走正规登录流程；密码、两步验证码必须由用户本人提供或确认，不要猜、不要绕。
3. 签名密钥是用户的重要资产：生成后把 keystore 文件和密码交给用户保存，并提醒其妥善保管（丢失后新版 APK 无法覆盖更新）。除填入用户自己仓库的 Secret 外，不要把密钥内容发到任何其他地方。
4. 每一步做完要回读核验（页面状态、API 返回、文件哈希），不要凭提交成功就宣布完成。

## 第 1 步：Fork

- 在浏览器打开上游仓库，点 Fork，目标为用户账号，仓库名保持 `HGhub`，Create fork。
- 成功标志：地址变为 `github.com/<用户账号>/HGhub`，页面显示 “forked from …”。
- 注意：某些 GitHub App / API 集成没有 fork 权限（会返回 403 Resource not accessible by integration），遇到时直接走网页 Fork，不要反复重试 API。

## 第 2 步：启用 Actions

- 打开 fork 仓库的 Actions 标签；若提示禁用，点 “I understand my workflows, go ahead and enable them”。
- 成功标志：页面显示 Actions 已启用，左侧能看到 Release 工作流。

## 第 3 步：生成安卓签名密钥（终端执行）

在你的工作目录里执行（密码随机生成并记录；别名固定 `hghub`；务必用 `-legacy` 以保证 apksigner 兼容）：

```bash
PW=$(openssl rand -hex 16)
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 3650 -nodes -subj "/CN=HGhub Personal"
openssl pkcs12 -legacy -export -out release.keystore -inkey key.pem -in cert.pem -name hghub -passout "pass:$PW"
base64 -w0 release.keystore > keystore-base64.txt   # macOS: base64 -i release.keystore -o keystore-base64.txt
rm -f key.pem cert.pem
```

产物：`release.keystore`、`keystore-base64.txt`（约 3264 字符、单行）、密码 `PW`。

## 第 4 步：填 4 个 Secret（浏览器操作）

入口：fork 仓库 Settings → Secrets and variables → Actions → New repository secret。逐个添加：

| Name | Value |
| --- | --- |
| `ANDROID_KEY_ALIAS` | `hghub` |
| `ANDROID_KEYSTORE_PASSWORD` | 第 3 步的密码 |
| `ANDROID_KEY_PASSWORD` | 同一个密码 |
| `ANDROID_KEY_BASE64` | `keystore-base64.txt` 的完整内容 |

**长字符串防错（本节是实测教训，务必照做）：**

- 禁止凭记忆手打或多次转抄这串 Base64：实测中手抄曾发生串行损坏。优先用文件直传/整段复制；工具链不支持读文件时，改用「分块传输」：把全文按 400 字符切块、逐块标注序号和首尾锚点发送，接收方拼完必须核对：总长 = 文件字符数、开头 = 文件前 15 字符、结尾 = 文件后 7 字符，三项全对才提交。
- 提交前如平台有安全确认，如实说明这是用户已授权的自有签名密钥并确认提交。
- 正确性最终以第 5 步签名步骤是否通过为准；失败先怀疑这串传输出错，重新传一遍再重跑，不要改工作流代码绕过签名。

## 第 5 步：触发流水线并监控

- Actions → Release → Run workflow（分支 main，无其他输入）→ 确认。
- 监控：可用公共 API 查询 `GET https://api.github.com/repos/<用户账号>/HGhub/actions/runs/<run_id>/attempts/<n>/jobs`（重跑后 attempt 号会变，别只看第一次尝试的结论）。
- 预期：Windows、macOS 约 6–8 分钟，Android 约 9–12 分钟。首次运行若 Android 报 “缺少 ANDROID_KEY_BASE64 secret，无法签名”，说明第 4 步没齐，补齐后在运行页 Re-run jobs → Re-run failed jobs 只重跑失败项。
- 成功标志：最新 attempt 下三个 job 全部 success（Publish GitHub Release 在手动触发时是 skipped，属正常）。

## 第 6 步：下载并交付产物

- 在运行页底部 Artifacts 下载 `windows`、`android`、`macos` 三个 zip（下载接口需要登录态，用浏览器下载）。
- 解压核对（文件名以版本号替换通配）：
  - windows：`HGhub_*_x64-setup.exe`（安装版）、`HGhub_*_x64_portable.zip`（便携版）
  - android：`HGhub_*_arm64.apk`（主流手机）、`HGhub_*_arm.apk`（老设备）、`HGhub_*_x86_64.apk`（模拟器）
  - macos：`HGhub_*_universal.dmg`
- 有条件时核对下载 zip 的 sha256 与运行页 Artifacts 显示值一致。
- 交付时说清：Windows 双击安装（SmartScreen 拦截时走「更多信息 → 仍要运行」）；安卓 APK 传到手机点装、允许一次未知来源；播放不了去 设置 → 线路 检测后换线。
- 最后把 `release.keystore` 与密码一并交给用户保存（这是后续版本覆盖更新的唯一钥匙），并说明其存放位置。

## 红线

- 仅限个人非商业使用（PolyForm Noncommercial）：不收费分发、不加广告、不做付费解锁。
- 不公开传播上游接口地址、播放直链与解密方式；不把产物或仓库拿去做公开宣传引流——原作者已明确表示希望低调维护、可能随时归档上游仓库，尊重这一意愿。
- 遇到上游仓库不可见/被归档：如实告诉用户，用用户 fork 里已有的源码继续即可，不要去找来路不明的镜像包。
