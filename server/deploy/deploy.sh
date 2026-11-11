#!/usr/bin/env bash
# HGhub Web 服务部署脚本（在服务器上以 root 或 sudo 执行）
# 用法：先把 hghub-web-linux.tar.gz 传到服务器任意目录，然后 sudo bash deploy.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then echo "请用 sudo 执行"; exit 1; fi

DIR=/opt/hghub-web
SRC="$(cd "$(dirname "$0")" && pwd)/hghub-web-linux.tar.gz"

# 专用系统用户
id hghub >/dev/null 2>&1 || useradd -r -s /usr/sbin/nologin hghub

# 解包（保留 .env 与数据目录）
mkdir -p "$DIR"
tar xzf "$SRC" -C "$DIR"
mkdir -p "$DIR/data"
touch "$DIR/.env"

# 监听地址：先只绑本机（127.0.0.1:8900），本地验通后改成 0.0.0.0:8900 再开云端端口
if ! grep -q '^HGHUB_WEB_ADDR=' "$DIR/.env" 2>/dev/null; then
  echo 'HGHUB_WEB_ADDR=127.0.0.1:8900' >> "$DIR/.env"
  echo "HGHUB_DATA_DIR=$DIR/data" >> "$DIR/.env"
  echo "HGHUB_DIST_DIR=$DIR/dist-web" >> "$DIR/.env"
fi

chmod +x "$DIR/hghub-web"
chown -R hghub:hghub "$DIR"

cp "$(dirname "$0")/hghub-web.service" /etc/systemd/system/hghub-web.service
systemctl daemon-reload
systemctl enable --now hghub-web

sleep 2
systemctl --no-pager status hghub-web | head -12
echo "---- 本地健康检查 ----"
curl -s http://127.0.0.1:8900/healthz && echo " (healthz OK)"
curl -s -o /dev/null -w "首页未登录: HTTP %{http_code}（应为 302 → /login）\n" http://127.0.0.1:8900/
