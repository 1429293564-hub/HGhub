# 小白指南：不用装任何开发环境，在网页上把安装包打出来

这个项目现在不在 Releases 里放现成安装包了，只剩源码。普通人想用，不需要在自己电脑上装一堆开发工具——项目自带了一条 GitHub Actions 打包流水线，你只要有一个 GitHub 账号，全程在网页上点几下，GitHub 的云端机器会替你把 Windows 安装包和安卓 APK 打好，你下载回来装上就行。

预计耗时：动手点按约 10 分钟，云端打包约 10–20 分钟。

> 先说清边界：本项目是个人技术实践作品，许可证为 PolyForm Noncommercial（非商业）。以下流程只供个人学习、自用。禁止把打出来的包出售、加广告、做付费解锁或任何商业分发；请保留仓库里的许可证与出处说明。原作者明确表示只想低调维护，请勿大肆宣传、传播上游接口或播放直链。

---

## 第 0 步：准备

- 一个 GitHub 账号（没有就先去 github.com 注册，免费）。
- 下面所有操作都在浏览器里完成，电脑、手机浏览器都可以（手机上建议切桌面版网页）。

## 第 1 步：Fork 到你自己的账号

1. 打开本仓库页面，点右上角 **Fork**。
2. 仓库名保持 `HGhub` 不用改，点 **Create fork**。
3. 跳转到 `你的账号/HGhub` 就对了。

Fork 的意思是把整份源码复制一份到你名下，后面所有操作都在你这份上面做，不会影响原仓库。

## 第 2 步：启用 Actions

1. 在你 fork 的仓库页面，点上方 **Actions** 标签。
2. 如果页面提示流水线被禁用，点 **I understand my workflows, go ahead and enable them** 启用。
3. 左侧能看到 **Release** 这条流水线，就是它负责打包。

## 第 3 步：生成你自己的安卓签名密钥，并填 4 个 Secret

这一步只影响安卓包：APK 必须签名才能装到手机上，而签名密钥不会从原仓库跟过来，需要你自己生成一把，并把它作为加密配置（Secret）填进你自己的仓库。Windows 和 macOS 包不需要这一步，如果你只要 Windows 版，可以跳过本步直接去第 4 步，只是安卓那一项会打包失败，不影响其他包。

### 3.1 生成密钥（任选一种方式）

**方式 A：用 JDK 的 keytool（Windows / macOS / Linux 通用，先装一个 JDK）**

在命令行里运行（把 `你的密码` 换成你自己定的一串密码，记牢，第 3.2 步要用三次；注意 `--ks-pass` 和 `--key-pass` 填同一个密码）：

```
keytool -genkeypair -keystore release.keystore -storetype PKCS12 -alias hghub -keyalg RSA -keysize 2048 -validity 3650 -storepass 你的密码 -keypass 你的密码 -dname "CN=HGhub Personal"
```

运行完会生成一个 `release.keystore` 文件。

**方式 B：用 OpenSSL（macOS / Linux 自带或易装）**

```
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 3650 -nodes -subj "/CN=HGhub Personal"
openssl pkcs12 -legacy -export -out release.keystore -inkey key.pem -in cert.pem -name hghub -passout pass:你的密码
rm key.pem cert.pem
```

生成的同样是一个 `release.keystore` 文件，密码就是你在命令里写的那串。

### 3.2 把密钥文件转成一串文本

Secret 里填的是这个文件的 Base64 文本，不是文件本身：

- macOS / Linux：`base64 -w0 release.keystore > keystore-base64.txt`（macOS 用 `base64 -i release.keystore -o keystore-base64.txt`）
- Windows（PowerShell）：`[Convert]::ToBase64String([IO.File]::ReadAllBytes("release.keystore")) > keystore-base64.txt`

得到一串很长的文本（约 3000 多个字符），下一步要用。**这一串和你的密码都要自己保存好**：以后这个 App 更新版本，必须用同一把密钥重新打包，手机上才能直接覆盖安装更新；密钥丢了就只能卸载重装。

### 3.3 填 4 个 Secret

在你 fork 的仓库里，依次打开 **Settings → Secrets and variables → Actions → New repository secret**，添加下面 4 个（Name 必须一字不差）：

| Name | 填什么 |
| --- | --- |
| `ANDROID_KEY_BASE64` | 第 3.2 步那一整串 Base64 文本（完整粘贴，不要截断、不要带空格换行） |
| `ANDROID_KEY_ALIAS` | `hghub` |
| `ANDROID_KEYSTORE_PASSWORD` | 第 3.1 步你定的密码 |
| `ANDROID_KEY_PASSWORD` | 同一个密码 |

Secret 提交后在网页上看不到原文，这是正常的。

## 第 4 步：运行 Release 流水线打包

1. 回到 **Actions** 标签，左侧点 **Release**。
2. 点 **Run workflow**，分支保持 `main`，再点绿色的 **Run workflow** 确认。
3. 等 10–20 分钟，三个任务并行打：**Windows、Android、macOS**。页面上全部变绿勾就是成功了。

如果总状态显示 Failure，先点进去看是哪一项红了：Windows 和 macOS 成功、只有 Android 红，大概率是第 3 步的 Secret 缺了或填错了——补好后在运行页右上角点 **Re-run jobs → Re-run failed jobs** 只重跑失败项，不用全部重来。

## 第 5 步：下载安装包

在那次运行记录页面的底部 **Artifacts** 区域，有 3 个压缩包，点哪个下哪个：

- **windows**：里面有 `HGhub_版本_x64-setup.exe`（安装版，双击安装）和 `HGhub_版本_x64_portable.zip`（便携版，解压直接运行）。
- **android**：里面有 3 个 APK，按手机型号选：
  - `arm64`：绝大多数安卓手机用这个；
  - `arm`：很老的 32 位手机用；
  - `x86_64`：电脑上的安卓模拟器用。
- **macos**：里面是 `.dmg`，苹果电脑用。

## 第 6 步：安装与使用

- **Windows**：双击 setup.exe 装好打开即可；系统 SmartScreen 若拦截，点「更多信息 → 仍要运行」（这是自打包软件的例行提示）。
- **安卓**：把 APK 传到手机点开安装，系统询问时允许一次「安装未知来源应用」。
- **使用**：打开 App 直接以访客身份浏览、播放即可，不需要会员。
- **播放不了 / 转圈**：去 **设置 → 线路**，先检测可达性，再换一条可用线路，这是最常见的问题和解法。

## 常见问题

- **Q：以后出新版本怎么更新？** 在你 fork 的仓库页面点 **Sync fork** 同步原仓库的最新代码，然后重复第 4、5 步。安卓务必继续用同一把签名密钥（Secret 还在就不用重填），这样手机上能直接覆盖更新。
- **Q：打包要钱吗？** 不要。GitHub Actions 对公开仓库的额度足够个人使用。
- **Q：我没有电脑，全程手机能操作吗？** 能。第 3.1 步生成密钥需要命令行，手机上不方便——这一步可以交给你的 AI 助手完成，见仓库里的 `DEPLOY.md`，或者直接把下面这段话发给你的 AI：

```
https://github.com/youyukongzhong/HGhub 帮我把这个仓库 fork 到我的 GitHub 账号，照着里面的 DEPLOY.md 打包出 Windows 和安卓安装包
```

- **Q：能不能把打好的包发给朋友 / 挂出去卖？** 发给朋友自用没问题；**出售、加广告、做成付费服务一律不行**，这是许可证的硬线。

---

维护说明：本指南是 fork 维护者补充的使用文档，只讲“怎么把包打出来”，不改变项目本身的定位与许可证。项目本体说明见 [README.md](README.md)，给 AI 助手的自动执行说明见 [DEPLOY.md](DEPLOY.md)。
