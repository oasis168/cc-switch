@echo off
chcp 65001 >nul 2>&1
cd /d "%~dp0.."
echo 正在启动 cc-switch...
pnpm tauri dev
pause
