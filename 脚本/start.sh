#!/bin/bash
cd "$(dirname "$0")/.."
echo "正在启动 cc-switch..."
pnpm tauri dev
