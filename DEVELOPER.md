# 大富翁3 Patch 開發者文件

> 這份文件給**要修改這個專案的人**。使用說明請看 [README.md](README.md)。
> 文件與發佈規範見 [docs/rules/](docs/rules/)。

---

## 1. 技術棧與系統需求

| 項目 | 版本 | 用途 |
| :--- | :--- | :--- |
| Python | 3.10 以上（實測 3.14） | 主程式 |
| tkinter | 隨 Python 內附 | 使用者介面 |
| `lunar_python` | 1.3.9 以上 | 農曆換算，產生 `Cald.b` |
| PyInstaller | 6.0 以上 | 打包成單一 EXE |
| Windows SDK（signtool） | 任意版本 | 數位簽章，選用 |

執行期唯一的第三方相依是 `lunar_python`；其餘只用標準函式庫 `tkinter` / `os` / `sys` / `shutil` / `re` / `datetime`。

**作業系統限制**：產物僅供 Windows。建置可在 Windows 進行，也可在 Linux/WSL 透過 Wine 執行（見 §7）。

---

## 2. 環境建置

1. 取得原始碼
   ```bash
   git clone git@github.com:iOvermind/RICH3_PATCH.git
   ```

2. 確認 Python 可用
   ```powershell
   python --version
   ```
   應顯示 3.10 以上版本。

3. 安裝相依套件
   ```powershell
   python -m pip install -r requirements.txt
   ```
   即使只是要執行 `main.py` 也需要這一步——`lunar_python` 是執行期相依。

4. 準備一份測試用的遊戲資料夾
   需要一份《大富翁3》的 `RICH3.EXE`、`MAP.MKF`、`SCREEN.MKF` 等檔案。**遊戲檔案不進版控**（見 §9.1），請自行放在庫外或已被忽略的目錄。

---

## 3. 日常開發

**直接執行**

```powershell
python main.py
```

視窗會開起來，行為與打包後**幾乎**相同——差別在資源釋放（見 §5 與 §10）：未打包時 `extract_bundled_folders()` 會偵測到不是 PyInstaller 環境而直接跳過，改用庫內現成的 `EVENTVOC/` `NEWSVOC/` `SCREEN/` 資料夾。

**修改後如何反映**：重新執行 `python main.py` 即可，沒有熱更新。

**除錯**：`emit_log()` 同時寫到終端機與 UI 日誌區。從終端機啟動就能看到完整輸出，格式為 `[狀態][STEP n/6] 訊息`。狀態字串有 `INFO` / `WARN` / `ERROR` / `SUCCESS` / `FATAL` / `DONE`，Rich Patch Series 兩支程式共用同一套。

日曆產生耗時數十秒。只想測其他步驟時，暫時把 `run_patch()` 裡的 `generate_calendars()` 註解掉會快很多——但**別忘了改回來**，第 3 步的 `total_days` 依賴它的回傳值。

---

## 4. 目錄結構

```text
RICH3_PATCH/
├─ main.py                  全部的程式碼：六個步驟 + tkinter 介面
├─ EVENTVOC/                內建事件語音資源，打包時一併嵌入
├─ NEWSVOC/                 內建新聞語音資源，打包時一併嵌入
├─ SCREEN/                  內建畫面資源，打包時一併嵌入
├─ rich3_patch.spec         PyInstaller 設定，含資源清單與瘦身排除清單
├─ file_version_info.txt    EXE 的版本資源（版本號在這裡）
├─ requirements.txt         相依套件
├─ build.ps1                Windows 打包腳本（建議用這支）
├─ build.bat                Windows 打包腳本（雙擊版）
├─ build.sh                 Linux/WSL 打包腳本（透過 Wine）
├─ icon.png / icon.ico      視窗圖示與 EXE 圖示
├─ github.bat / github.sh   推送輔助腳本
└─ docs/rules/              文件與發佈規範（正典在 DEV_TEMPLATE）
```

`main.py` 以註解分為四段：共用工具、核心處理（六個步驟各一組函式）、主幹邏輯（`run_patch`）、介面（`main`）。

---

## 5. 架構與關鍵設計決策

### 六個步驟

`run_patch()` 依序執行，`TOTAL_STEPS = 6`：

| 步驟 | 函式 | 做什麼 |
| :-: | :--- | :--- |
| 1 | `extract_bundled_folders()` | 把內建的 `EVENTVOC` / `NEWSVOC` / `SCREEN` 釋放到遊戲目錄 |
| 2 | `generate_calendars()` | 產生 `Cald.a`（國曆）與 `Cald.b`（農曆），共 14612 天 |
| 3 | `patch_exe()` | `RICH3.EXE` 的 14 條特徵碼（含 1 條 regex 型，依日曆天數動態產生） |
| 4 | `patch_map_mkf()` | `MAP.MKF` 的 2 條物價修正 |
| 5 | `patch_screen_mkf()` | `SCREEN.MKF` 的索引表拆解與重組 |
| 6 | `patch_audio_mkf()` | `NEWSVOC` 與 `EVENTVOC` 兩個語音 MKF 的注入 |

### 關鍵決策

#### 日曆以執行當下為基準動態產生

- **決定**：`generate_calendars()` 取執行當下年份 `-10` 為起點，往後 14612 天（40 年），並把天數回傳給第 3 步寫進 EXE 的搜尋組數。
- **理由**：原版日曆是固定資料，到 2018/6/6 之後農曆就崩潰。改成動態產生後，使用者任何時候重跑都能把有效範圍往後推。
- **代價**：**產物不可重現**——同一份遊戲檔在不同日期執行會得到不同的 `Cald.a` / `Cald.b`。做輸出比對時必須固定基準日期（見 §10）。

#### MKF 走「拆索引表 → 改內容 → 重建索引表」

- **決定**：`read_mkf_chunks()` / `write_mkf_chunks()` 把 MKF 拆成區塊再重組，而不是就地覆寫。
- **理由**：注入的語音長度與原本不同，索引表的偏移量必須整體重算。
- **代價**：任何對 MKF 處理的修改都要同時顧到索引表，改壞了遊戲會直接讀不到資源。

#### 特徵碼比對：EXE 只換第一次，MKF 全部換

- **決定**：`patch_binary()` 對非 `.MKF` 檔案只替換第一個符合位置，對 `.MKF` 則全域替換。
- **理由**：EXE 內特徵碼是特定指令位置，全域替換可能誤傷；MKF 是資料檔，同一筆數值可能合法地出現多次且都該改。
- **代價**：判斷依據是**副檔名**，新增其他類型的目標檔時要留意這個分支。

#### 備份不覆蓋

- **決定**：`backup_file()` 只在 `.bak` 不存在時建立備份。
- **理由**：使用者重複執行是常態。若每次都覆蓋備份，第二次執行後就再也回不到原版。
- **代價**：使用者若手動改壞了 `.bak`，程式不會察覺。

---

## 6. 測試

**目前沒有自動化測試。** 驗證靠手動比對。

建議的驗證流程：

1. 準備一份未修改的遊戲資料夾副本，記下所有將被修改檔案的雜湊值：
   ```powershell
   Get-FileHash .\RICH3.EXE, .\MAP.MKF, .\SCREEN.MKF, .\NEWSVOC.MKF, .\EVENTVOC.MKF -Algorithm SHA256
   ```
2. 執行 `python main.py`，選到該資料夾並按開始。
3. 確認六個步驟都跑完，摘要視窗五個項目都顯示成功。
4. 確認每個被修改的檔案旁邊都出現了對應的 `.bak`，且 `.bak` 的雜湊值等於步驟 1 的原始值。
5. 確認 `Cald.a` 為 `14612 × 4 = 58448` 位元組，`Cald.b` 同樣大小。
6. 再執行一次，確認 `.bak` 沒有被覆蓋。
7. 進遊戲確認：不需光碟或顏色密碼即可啟動、多人地圖可單人開局、農曆顯示正確、修正過的獎金與物價數值正確、補上的語音會播放。

**遷移到新實作時的驗證基準**：本 Python 版是驗證未來 Tauri 版正確性的 oracle。比對必須是**逐位元組相同**，特別是 `Cald.a` / `Cald.b` 的 14612 天資料——農曆換算的實作差異不接受「差幾天而已」。詳見 [TAURI_MIGRATION.md](TAURI_MIGRATION.md) 步驟 5，以及 §10 關於固定基準日期的說明。

---

## 7. 建置與產物

**Windows**

```powershell
.\build.ps1                # 打包
.\build.ps1 -Sign          # 打包並簽章
.\build.ps1 -SkipDeps      # 跳過套件檢查，較快
```

也可以雙擊 `build.bat`（功能相同，但一定會嘗試簽章）。

**Linux / WSL**

```bash
./build.sh                 # 透過 Wine 呼叫 Windows 版 Python
```

簽章改用原生 `osslsigncode`（`sudo apt install osslsigncode`），不走 Wine + signtool。

**產物**

| 產物 | 用途 |
| :--- | :--- |
| `dist/rich3_patch.exe` | 單一執行檔，免安裝。內含約 440 KB 的語音與畫面資源。目前唯一的發佈形式。 |

打包**一律走 `rich3_patch.spec`**，不要直接下 `pyinstaller main.py`——`.spec` 裡的資源清單（`EVENTVOC` / `NEWSVOC` / `SCREEN`）與瘦身設定只有走 `.spec` 才會生效（見 §10）。

> **已知落差**：建置腳本目前輸出 `rich3_patch.exe`，尚未符合
> [docs/rules/RELEASE_RULES.md](docs/rules/RELEASE_RULES.md) §2.1 要求的
> `[RICH3_PATCH][v1.0.0][Portable].exe` 格式。發佈前必須改名，或調整建置腳本。

### 版本號

**單一來源**：`file_version_info.txt`

| 位置 | 欄位 | 方式 |
| :--- | :--- | :--- |
| `file_version_info.txt` | `filevers` / `prodvers` | 手動（單一來源，四元組如 `(1, 0, 0, 0)`） |
| `file_version_info.txt` | `FileVersion` / `ProductVersion` | 手動（字串如 `1.0.0`，須與上者一致） |
| EXE 版本資源 | — | 自動（PyInstaller 由 `.spec` 的 `version=` 讀入） |

發佈前依 [docs/rules/VERSION_RULES.md](docs/rules/VERSION_RULES.md) §7 逐項核對。

---

## 8. 分支、commit 與 PR 慣例

- **主分支**：`main`
- **開分支**：從 `main` 開，功能用 `feat/<描述>`、修正用 `fix/<描述>`。
- **commit 訊息**：首行為繁中祈使句摘要，必要時空一行後補理由。

### 舊實作的保留

本專案正在遷移到 Rust + Tauri（見 [TAURI_MIGRATION.md](TAURI_MIGRATION.md)）。`main` 走 Tauri 版，Python + tkinter 版依 `docs/rules/DEVELOPER_RULES.md` §4.3 以分支保留：

| 分支 | 內容 | 保留原因 | 解除條件 |
| :--- | :--- | :--- | :--- |
| `legacy/python-tkinter` | 遷移前的完整 Python + tkinter 實作，含 `lunar_python` 的農曆換算 | 是驗證 Tauri 版正確性的基準（oracle）。**農曆換算是最大風險**——Rust 端的候選函式庫是不同實作，`Cald.a` / `Cald.b` 的 14612 天資料必須逐位元組相同，不接受「差幾天而已」 | 14612 天比對通過（比對時兩邊須固定同一個基準日期，見 §10），且 Tauri 版實機驗收完成後 |

**該分支不再接受新功能**，僅在有明確理由時接受修正。**不得刪除**。

---

## 9. 安全與敏感資料

### 9.1 機密不進版控

| 項目 | 排除方式 | 本機該放哪 |
| :--- | :--- | :--- |
| 簽章憑證 `Overmind.pfx` | `.gitignore` 的 `*.pfx` | 專案根目錄，由建置腳本自動產生 |
| 遊戲原始檔（`*.EXE` / `*.MKF` / `*.PAT` / `*.a` / `*.b` / `*.bak`…） | `.gitignore` 逐項排除 | 庫外，或 `original/` / `dist/` 等已忽略的目錄 |

⚠ **憑證密碼 `overmind` 以明文寫在 `build.ps1`、`build.bat`、`build.sh` 中。** 這是刻意的權衡：該憑證為自簽、僅用於讓 EXE 帶上發行者名稱，本身不具信任價值，密碼公開不造成額外風險。**因此這張憑證不得用於任何其他用途**；若日後改用有實際信任價值的憑證，密碼必須改由環境變數或憑證存放區提供。

**注意**：`EVENTVOC/`、`NEWSVOC/`、`SCREEN/` 是本專案自製的資源（語音以 GPT-SoVITS 合成），**刻意進版控**；`.gitignore` 排除的 `*.MKF` 是遊戲原始檔，兩者不要搞混。

### 9.2 權限最小化

| 要求的權限 | 為什麼需要 |
| :--- | :--- |
| 讀寫使用者選定資料夾內的檔案 | patch 的本質就是改寫遊戲檔案 |
| 在使用者選定資料夾內建立/刪除子資料夾 | 第 1 步要釋放內建資源，結束後清除（見 §10） |

**刻意不要的權限**：程式不連網、不讀寫使用者選定目錄以外的位置、不寫登錄檔、不需要管理員權限。`.spec` 的排除清單裡明確排掉了 `socket`、`ssl`、`urllib`、`http`——這同時是瘦身也是保證。

### 9.3 依賴來源與鎖檔

- 執行期相依只有 `lunar_python`（PyPI），版本下限寫在 `requirements.txt`；沒有鎖檔。
- **`lunar_python` 的換算結果直接決定遊戲內農曆是否正確**，升版本後必須重新驗證（§6 步驟 7）。
- 打包相依只有 PyInstaller。

### 9.4 破壞性操作的保護

| 操作 | 影響的資料 | 可回復機制 |
| :--- | :--- | :--- |
| 改寫 `RICH3.EXE`、`MAP.MKF`、`SCREEN.MKF`、兩個語音 MKF | 使用者的遊戲檔案 | 先複製為 `<原檔名>.bak`；**若 `.bak` 已存在則不覆蓋**。還原方式為把 `.bak` 改名回原檔名。 |
| 覆寫 `Cald.a` / `Cald.b` | 遊戲日曆檔 | 同上，首次執行時備份。注意日曆**每次執行都會重新產生**，這是刻意的。 |
| 釋放資源資料夾到遊戲目錄 | `EVENTVOC` / `NEWSVOC` / `SCREEN` 三個資料夾 | 程式建立的會在 `finally` 區塊清除；**使用者原本就有的同名資料夾會被覆寫且不會被清除**（見 §10） |

修改任何會寫入使用者檔案的程式碼時，都必須維持「備份優先、不覆蓋既有備份」的原則。

---

## 10. 已知陷阱

#### 14 條 EXE 特徵碼不會全部命中

- **症狀**：日誌顯示 `已儲存修改 (11/14 項)`，其中「修正住院/坐牢免付過路費位置」、「破解顏色密碼 (磁片版)」、「破解光碟檢查 (相容項 1)」顯示跳過。
- **原因**：磁片版、重訂光碟版、Steam 典藏版的偏移位址不同，程式把各版本的特徵碼全部列出逐一嘗試。任一版本本來就只會命中屬於它的那幾條。
- **處置**：這是正常行為。以 Steam 典藏版實測即為 **11/14**。判斷成功與否要看關鍵項目（免光碟、日曆、獎金）有沒有命中，不是看是否全中。

#### 日曆檔每次執行的內容都不同，無法用固定雜湊比對

- **症狀**：同一份遊戲檔跑兩次，`Cald.a` / `Cald.b` 的 SHA-256 不一樣。
- **原因**：`generate_calendars()` 以 `datetime.datetime.now()` 取當下年份 `-10` 為起點，換一天執行就是不同的資料。這是功能不是錯誤。
- **處置**：做輸出比對（例如驗證未來的 Rust 實作）時，**必須固定基準日期**——暫時把 `now` 改成寫死的日期，兩邊用同一個基準跑，比完再改回來。直接比對兩次不同時間的執行結果一定會失敗。

#### 使用者原本就有的資源資料夾不會被清除

- **症狀**：跑完之後，遊戲目錄裡多出 `EVENTVOC` / `NEWSVOC` / `SCREEN` 資料夾沒有被清掉。
- **原因**：`extract_bundled_folders()` 只把**自己新建**的路徑記進 `temp_extracted`；若目標資料夾原本就存在，走的是 `dirs_exist_ok=True` 覆寫分支，不會登記，因此 `cleanup_folders()` 也不會刪它。
- **處置**：這是刻意的保守行為——不刪使用者原本就有的東西。改動這段邏輯時務必維持這個界線，別讓程式刪掉不是自己建立的目錄。

#### 直接用 `pyinstaller main.py` 打包，資源不會被嵌入

- **症狀**：打包成功，但執行後第 1 步找不到內建資源，或語音沒有被補上。
- **原因**：`EVENTVOC` / `NEWSVOC` / `SCREEN` 是寫在 `rich3_patch.spec` 的 `datas=` 裡的；直接指定 `main.py` 會用預設設定重新產生一份 spec，資源清單與瘦身排除清單全部失效。
- **處置**：一律 `python -m PyInstaller --clean --noconfirm rich3_patch.spec`，或直接用 `build.ps1`。

#### 開發時直接跑 `python main.py`，資源釋放步驟會被跳過

- **症狀**：終端機顯示「非 PyInstaller 打包環境，使用本地資料夾。」，第 1 步什麼也沒做。
- **原因**：`extract_bundled_folders()` 以 `sys._MEIPASS` 是否存在判斷是否為打包環境；未打包時直接返回。
- **處置**：這是正常行為。要驗證資源釋放與清除邏輯，**必須實際打包後測試**，不能只跑原始碼。

#### 砍掉 Tcl 的 encoding 後程式啟動就閃退

- **症狀**：打包後的 EXE 雙擊沒反應或瞬間關閉，從終端機執行可看到 Tcl 初始化相關錯誤。
- **原因**：`.spec` 的 `KEEP_ENCODINGS` 只保留了必要的編碼檔。若把 `ascii.enc`、`utf-8.enc`、`unicode.enc`、`cp950.enc` 之類移除，Tcl 啟動或繁中環境就會失敗。
- **處置**：調整 `KEEP_ENCODINGS` 後**必須**實際執行打包出來的 EXE 驗證，不能只看打包有沒有成功。

#### 在 Linux 上用 Wine 打包，簽章步驟會卡住

- **症狀**：`build.sh` 執行到簽章時無回應或報錯。
- **原因**：Wine 底下跑 Windows 版 `signtool.exe` 極不穩定。
- **處置**：`build.sh` 已改用 Linux 原生的 `osslsigncode`。沒安裝的話腳本會提示並跳過簽章，產物仍然可用。

---

## 相關文件

- 使用說明：[README.md](README.md)
- 變更紀錄：[CHANGELOG.md](CHANGELOG.md)
- 遷移計畫：[TAURI_MIGRATION.md](TAURI_MIGRATION.md)
- 文件與發佈規範：[docs/rules/](docs/rules/)
