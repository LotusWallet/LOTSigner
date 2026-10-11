@echo off
rem Lotus Signer 网页端一键启动（Windows）
rem 用法：双击本文件，浏览器打开 http://127.0.0.1:4180
rem 本脚本只启动一个本地静态文件服务，不发起任何外部连接。

cd /d "%~dp0"
echo.
echo   Lotus Signer web - http://127.0.0.1:4180
echo   按 Ctrl+C 停止
echo.
start "" http://127.0.0.1:4180/index.html
"C:\Users\simon\.workbuddy\binaries\python\versions\3.13.12\python.exe" -m http.server 4180 --bind 127.0.0.1
