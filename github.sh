#!/bin/bash
# 設定編碼 (Linux 預設通常已是 UTF-8)

REPO_NAME="RICH3_PATCH"
# 🔑 直接把 SSH 遠端網址寫死在這裡
REPO_URL="git@github.com:iOvermind/RICH3_PATCH.git"

echo "=================================="
echo "   🚀 Git 自動化助手 (SSH 終極懶人版)"
echo "   💻 Repo: $REPO_NAME"
echo "=================================="

# 🌟 關鍵防呆：如果沒有 .git 資料夾，自動幫你初始化
if [ ! -d ".git" ]; then
    echo "📁 偵測到這還不是 Git 專案，正在自動初始化 (git init)..."
    git init
    # 強制把預設分支命名為 main，避免舊版 git 預設 master 造成衝突
    git branch -m main 2>/dev/null || git checkout -b main 2>/dev/null
    echo "✅ 初始化完成！"
    echo "----------------------------------"
fi

echo "請選擇功能："
echo "  [1] ⬇️  下載更新 (Pull)"
echo "  [2] ⬆️  上傳備份 (Push)"
echo "=================================="
read -p "請輸入選項 [1/2]: " choice

# 🌟 終極 SSH 查勤：不要相信管家，直接戳 GitHub 測試連線！
    ssh -T -o BatchMode=yes git@github.com 2>&1 | grep -q "successfully authenticated"
    if [ $? -ne 0 ]; then
        echo "⏳ 偵測到 SSH 鑰匙已上鎖或管家卡陰！"
        echo "🧹 正在強制清除管家殘留的智障記憶..."
        ssh-add -D >/dev/null 2>&1
        echo "🔐 請重新輸入密碼解鎖 (將自動為您記住 1 小時)："
        ssh-add -t 3600 ~/.ssh/id_ed25519
        
        if [ $? -ne 0 ]; then
            echo "❌ 解鎖失敗！腳本終止。"
            exit 1
        fi
        echo "----------------------------------"
    fi

if [ "$choice" == "1" ]; then
    echo "----------------------------------"
    echo "⬇️  正在對著專屬網址拉取更新..."
    git pull "$REPO_URL" main
    echo "✅ 下載完成！"

elif [ "$choice" == "2" ]; then
    echo "----------------------------------"
    echo "📦 正在準備上傳程序..."

    # 註：requirements.txt 改為手動維護，不再 pip freeze
    #     整包環境倒進去會讓 PyInstaller 誤抓一堆套件，EXE 直接肥好幾 MB。

    # 加入所有變更
    git add .
    
    # 🌟 修正點：把 Commit 跟 Push 的判斷完全拆開
    if [ -z "$(git status --porcelain)" ]; then
        echo "⚠️  工作區沒有新變更，跳過 Commit 步驟..."
    else
        read -p "請輸入 Commit 訊息 (直接 Enter 自動填時戳): " input_msg
        if [ -z "$input_msg" ]; then
            dt=$(date +"%Y-%m-%d %H:%M:%S")
            commit_msg="Auto update: $dt"
        else
            commit_msg="$input_msg"
        fi
        git commit -m "$commit_msg"
    fi

    # 不管剛剛有沒有 Commit，最後都強制推送到遠端檢查一下！
    echo "☁️  正在推送到 GitHub..."
    git push "$REPO_URL" main
    
    if [ $? -eq 0 ]; then
        echo "✅ 上傳搞定！收工！"
    else
        echo "❌ 上傳失敗！請檢查網路或遠端是否有衝突。"
    fi

else
    echo "❌ 選項錯誤。"
fi

echo ""
read -p "請按任意鍵繼續..." -n 1 -s
echo ""