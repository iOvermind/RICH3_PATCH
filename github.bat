@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

:: ==========================================
:: 0. 基本設定
:: ==========================================
set REPO_NAME=RICH3_PATCH
:: 🔑 直接把 SSH 遠端網址寫死在這裡
set REPO_URL=git@github.com:iOvermind/RICH3_PATCH.git

echo ==================================
echo    🚀 Git 自動化助手 (Windows 家用版)
echo    💻 Repo: %REPO_NAME%
echo ==================================

:: 🌟 關鍵防呆：如果沒有 .git 資料夾，自動幫你初始化
if not exist ".git\" (
    echo 📁 偵測到這還不是 Git 專案，正在自動初始化...
    git init
    git branch -m main 2>nul || git checkout -b main 2>nul
    echo ✅ 初始化完成！
    echo ----------------------------------
)

echo 請選擇功能：
echo   [1] ⬇️  下載更新 (Pull)
echo   [2] ⬆️  上傳備份 (Push)
echo ==================================
set /p choice="請輸入選項 [1/2]: "

:: ==========================================
:: 2. 執行邏輯
:: ==========================================
if "%choice%"=="1" (
    echo ----------------------------------
    echo ⬇️  正在對著專屬網址拉取更新...
    git pull "%REPO_URL%" main
    echo ✅ 下載完成！

) else if "%choice%"=="2" (
    echo ----------------------------------
    echo 📦 正在準備上傳程序...

    :: 註：requirements.txt 改為手動維護，不再 pip freeze
    ::     整包環境倒進去會讓 PyInstaller 誤抓一堆套件，EXE 直接肥好幾 MB。

    :: 加入所有變更
    git add .

    :: 🌟 修正點：把 Commit 跟 Push 的判斷完全拆開
    set "HAS_CHANGES="
    for /f %%i in ('git status --porcelain') do set HAS_CHANGES=1

    if not defined HAS_CHANGES (
        echo ⚠️  工作區沒有新變更，跳過 Commit 步驟...
    ) else (
        set /p input_msg="請輸入 Commit 訊息 (直接 Enter 自動填時戳): "
        if "!input_msg!"=="" (
            for /f "usebackq" %%i in (`powershell -NoProfile -Command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"`) do set dt=%%i
            set commit_msg=Auto update: !dt!
        ) else (
            set commit_msg=!input_msg!
        )
        git commit -m "!commit_msg!"
    )

    :: 不管剛剛有沒有 Commit，最後都強制推送到遠端檢查一下！
    echo ☁️  正在推送到 GitHub...
    git push "%REPO_URL%" main

    if !errorlevel! equ 0 (
        echo ✅ 上傳搞定！收工！
    ) else (
        echo ❌ 上傳失敗！請檢查網路或遠端是否有衝突。
    )

) else (
    echo ❌ 選項錯誤。
)

echo.
pause
