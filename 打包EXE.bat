@echo off
cd /d "%~dp0"
npm.cmd install
if errorlevel 1 goto error
npm.cmd run build
if errorlevel 1 goto error
if not exist "dist" mkdir "dist"
copy /y "src-tauri\target\release\one-click-launcher.exe" "dist\一键启动器.exe" >nul
echo.
echo 打包完成：dist\一键启动器.exe
pause
exit /b 0
:error
    echo.
    echo 打包失败，请检查上方错误信息。
    pause
    exit /b 1
