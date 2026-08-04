@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

:: Rich Patch Series — Windows 打包腳本

set APP_NAME=rich3_patch
set APP_LABEL=Richman 3 Patch

echo ==================================
echo   [+] %APP_LABEL% Builder Pro
echo   [+] Author: Overmind
echo ==================================

:: 1. 清理舊檔案
echo [*] Cleaning old files...
if exist "build" rmdir /s /q "build"
if exist "dist" rmdir /s /q "dist"

:: 2. 建立自簽憑證 (單獨呼叫 PowerShell)
if not exist "Overmind.pfx" (
    echo [*] Generating Certificate...
    powershell -Command "$cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=Overmind' -KeyExportPolicy Exportable -KeySpec Signature -KeyLength 2048 -KeyAlgorithm RSA -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(10) -CertStoreLocation 'Cert:\CurrentUser\My'; $pwd = ConvertTo-SecureString -String 'overmind' -Force -AsPlainText; Export-PfxCertificate -Cert $cert -FilePath '.\Overmind.pfx' -Password $pwd"
    echo [OK] Overmind.pfx created.
)

:: 3. 打包 EXE
::    一律走 .spec 檔，資源清單與瘦身用的 EXCLUDES 才會生效
echo [*] Building EXE with PyInstaller...
python -m PyInstaller --clean --noconfirm "%APP_NAME%.spec"

if %errorlevel% neq 0 (
    echo [ERROR] PyInstaller failed! 屁啦，檢查一下 Python 套件。
    pause
    exit /b
)

:: 4. 數位簽章
echo [*] Signing the executable...
set "SIGNTOOL=C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe"

if exist "%SIGNTOOL%" (
    "%SIGNTOOL%" sign /f "Overmind.pfx" /p "overmind" /fd SHA256 /t http://timestamp.digicert.com /d "%APP_LABEL%" /v "dist\%APP_NAME%.exe"
) else (
    echo [WARN] 找不到 signtool.exe，跳過簽章。
)

:: 5. 回報成品體積，隨時盯著不要肥起來
if exist "dist\%APP_NAME%.exe" (
    for %%F in ("dist\%APP_NAME%.exe") do echo [SIZE] %%~zF bytes
)

echo.
echo [DONE] 完工！請到 dist 資料夾查看 %APP_NAME%.exe
pause
