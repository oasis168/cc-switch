@echo off
chcp 65001 >nul 2>&1
cd /d "%~dp0.."
echo 正在安装 cc-switch 依赖...
pnpm install
pause
