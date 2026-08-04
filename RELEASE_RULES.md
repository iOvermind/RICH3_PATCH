# 專案 Release 發佈規範 (Release Guidelines)

## 1. 檔案命名規範

所有對外發佈的編譯打包檔、安裝包或免安裝壓縮包，**必須嚴格保留中括號 `[]`**，格式如下：

`[專案名稱][版本號][Setup/Portable]`

### 格式說明：
* **`[專案名稱]`**：專案識別名稱，保留中括號（如 `[RICH3_PATCH]`）
* **`[版本號]`**：帶 v 之語意化版本號，保留中括號（如 `[v1.0.0]`）
* **`[Setup/Portable]`**：安裝版標記為 `[Setup]`，免安裝版標記為 `[Portable]`

### 實體範例：
* **安裝版**：`[RICH3_PATCH][v1.0.0][Setup].exe`
* **免安裝版**：`[RICH3_PATCH][v1.0.0][Portable].zip`

---

## 2. 版本號規範 (VERSION.md)

本專案嚴格遵循語意化版本號 `vMAJOR.MINOR.PATCH`：
* **MAJOR (主版本號)**：不向下相容的重大變更、架構重構（如 `v2.0.0`）。
* **MINOR (次版本號)**：向下相容的新功能新增或模組擴充（如 `v1.1.0`）。
* **PATCH (修訂號)**：向下相容的 Bug 修復、安全性補丁（如 `v1.0.1`）。

---

## 3. 發佈門檻 (Release Criteria)

在執行任何 Release 前，必須滿足以下條件：
- [ ] 所有預計發佈的變更均已 Merge 至 `main` 分支。
- [ ] 程式碼編譯成功，且基本功能測試通過。
- [ ] `CHANGELOG.md` 已更新當前版本的變更紀錄。
- [ ] 確保打包出的檔案名稱，**完全符合**保留中括號的命名格式。

---

## 4. 標準發佈流程 (Release Checklist)

### Step 1：本機打包與檢驗
執行編譯打包，並仔細檢查產出物名稱是否帶有完整中括號。
```text
# 產出確認範例
[RICH3_PATCH][v1.0.0][Portable].zip
[RICH3_PATCH][v1.0.0][Setup].exe
```

---

## 5. 建立 Git Tag 並推送

Git Tag 的慣例維持單純帶 v 即可（Tag 本身不需要加中括號）：

```powershell
# 1. 建立標籤
git tag -a v1.0.0 -m "Release v1.0.0"
# 2. 推送標籤至遠端
git push origin v1.0.0
```

---

## 6. 建立 GitHub Release 頁面

1. 至 GitHub Releases 頁面點選 `Draft a new release`。
2. 選擇剛推送的 Tag（如 `v1.0.0`）。
3. 標題填寫 `[專案名稱] v1.0.0 Release`（如 `[RICH3_PATCH] v1.0.0 Release`）。
4. 內文貼上本次更新的 `CHANGELOG.md` 說明。
5. 上傳附件：將嚴格依規範命名的打包檔上傳至附件區。
6. 點選 `Publish release` 完成發佈。