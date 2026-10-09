@echo off
cd /d "%~dp0"
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
cargo --version
if not exist node_modules call npm install
call npm run tauri dev > dev-log.txt 2>&1
echo EXITED %ERRORLEVEL% >> dev-log.txt
