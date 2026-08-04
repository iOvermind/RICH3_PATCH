#!/bin/bash
# Rich Patch Series — Linux/WSL 打包腳本 (透過 Wine 呼叫 Windows Python)

APP_NAME="rich3_patch"
APP_LABEL="Richman 3 Patch"

echo "=================================="
echo "  [+] $APP_LABEL Builder Pro"
echo "  [+] Author: Overmind"
echo "=================================="

# 1. 清理舊檔案
echo "[*] Cleaning old files..."
rm -rf build dist

# 2. 建立自簽憑證 (改用 Linux 原生 OpenSSL 產生 PFX，比 Wine 跑 PowerShell 穩太多了)
if [ ! -f "Overmind.pfx" ]; then
    echo "[*] Generating Certificate..."
    # 先產生 key 和 crt
    openssl req -x509 -newkey rsa:2048 -keyout temp_key.pem -out temp_cert.pem -days 3650 -nodes -subj "/CN=Overmind" 2>/dev/null
    # 打包成 pfx，密碼設定為 overmind
    openssl pkcs12 -export -out Overmind.pfx -inkey temp_key.pem -in temp_cert.pem -passout pass:overmind 2>/dev/null
    # 毀屍滅跡
    rm temp_key.pem temp_cert.pem
    echo "[OK] Overmind.pfx created."
fi

# 3. 打包 EXE
#    一律走 .spec 檔，資源清單與瘦身用的 EXCLUDES 才會生效
echo "[*] Building EXE with PyInstaller..."
wine python -m PyInstaller --clean --noconfirm "${APP_NAME}.spec"

if [ $? -ne 0 ]; then
    echo "[ERROR] PyInstaller failed! 屁啦，檢查一下 Python 套件。"
    exit 1
fi

# 4. 數位簽章 (放棄 Wine + signtool，改用 Linux 原生 osslsigncode)
echo "[*] Signing the executable..."

# 檢查系統有沒有裝 osslsigncode
if command -v osslsigncode &> /dev/null; then
    # 執行原生簽章
    osslsigncode sign -pkcs12 "Overmind.pfx" -pass "overmind" \
        -n "$APP_LABEL" \
        -t http://timestamp.digicert.com \
        -in "dist/${APP_NAME}.exe" \
        -out "dist/${APP_NAME}_signed.exe"

    if [ $? -eq 0 ]; then
        # 簽章成功就把原本未簽章的覆蓋掉
        mv "dist/${APP_NAME}_signed.exe" "dist/${APP_NAME}.exe"
        echo ""
        echo "[DONE] 完工！簽章完美打上，請到 dist 資料夾查看 ${APP_NAME}.exe"
    else
        echo "[WARN] 簽章過程報錯，請檢查憑證或網路連線。"
    fi
else
    echo "[WARN] 靠背，你沒裝 osslsigncode 啦！請先去終端機跑 sudo apt install osslsigncode"
    echo "[DONE] 程式已打包，但未上數位簽章。"
fi

# 5. 回報成品體積，隨時盯著不要肥起來
if [ -f "dist/${APP_NAME}.exe" ]; then
    echo "[SIZE] $(du -h "dist/${APP_NAME}.exe" | cut -f1)"
fi
