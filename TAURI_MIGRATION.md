# Rich Patch Series — 遷移到 Rust + Tauri 計畫書

> 狀態：**待審核**。本文件寫完即停，尚未動工。
> 撰寫日期：2026-08-04
> 涵蓋專案：`RICH2_PATCH`、`RICH3_PATCH`

---

## 1. 為什麼要換

現行兩支 patcher 是 Python + tkinter，用 PyInstaller onefile 打包。體積是唯一但致命的問題：

| 產物 | 目前體積 | 組成 |
| :--- | ---: | :--- |
| `rich2_patch.exe` | 9.70 MB | python314.dll 6.44 MB + Tcl/Tk 3.3 MB + 程式碼數十 KB |
| `rich3_patch.exe` | 10.17 MB | 同上，另含 440 KB 內建語音/畫面資源 |

已經做過的瘦身（`.spec` 的 `EXCLUDES`、`optimize=2`、砍掉 Tcl 的 `tzdata`/`encoding`/`msgs`）只從 10.45 MB 降到 9.70 MB。**剩下的體積幾乎全是 Python 直譯器與 Tcl/Tk 執行期本身，在這個技術棧下已經沒有空間再壓。**

對照組就在隔壁：`RICH2_EDITOR` 走 Tauri 2，帶著完整的 Vite + Tailwind UI，portable exe 只有 **3.86 MB**，NSIS 安裝檔 1.65 MB。

換過去的預期收益：

- 體積約 **3.5–4 MB**（RICH3 加上 440 KB 資源約 4.3 MB），砍掉六成。
- UI 與 `RICH2_EDITOR` 同一套視覺語言，「系列感」比 tkinter 強得多。
- 二進位處理放 Rust，型別與錯誤處理都比現在的 Python 嚴謹。

---

## 2. 已確認的環境（本機實測，不需另外安裝）

| 項目 | 版本 |
| :--- | :--- |
| Rust / cargo | 1.97.1 |
| Node | v25.8.1 |
| npm | 11.11.0 |
| WebView2 Runtime | 150.0.4078.105 |
| crates.io / npm registry | 皆可連線 |

`RICH2_EDITOR/src-tauri` 已有可直接沿用的 Tauri 2 設定範本。

---

## 3. 已拍板的決策

| 決策 | 選擇 | 理由 |
| :--- | :--- | :--- |
| 專案結構 | **兩支獨立 app** | 維持現有兩個 GitHub repo 與 `github.sh`/`github.bat` 流程不變，發佈兩個 exe。 |
| 舊版去留 | **保留 Python 版到驗證通過** | `main.py` 是驗證 Tauri 版正確性的 oracle，特別是農曆換算。驗證完才移除。 |
| patch 引擎位置 | **Rust（`src-tauri`）** | 與 `RICH2_EDITOR`（邏輯放前端 TS）刻意不同。patcher 是二進位改檔，放 Rust 可避免給前端開放整個檔案系統權限，二進位處理也乾淨。前端只負責畫面與事件。 |
| 版本號 | **Tauri 版即為 v1.0.0** | 不跳 2.0.0。三個專案（含 `RICH2_EDITOR`）統一發佈 v1.0.0。 |
| 舊 release | **驗證通過後直接取代** | 現有 release 與 tag 不保留，一律砍掉重發。 |

---

## 4. 目標架構

兩個專案結構完全對稱，只有 `src-tauri/src/patch/` 內容不同：

```
RICH2_PATCH/
  index.html
  package.json
  tsconfig.json
  vite.config.ts
  src/
    main.ts                前端進入點：綁事件、收 Rust 送來的日誌/進度
    style.css              Tailwind 設定與色票（與 editor 同一份 @theme）
    ui/log-view.ts         日誌區
    ui/progress.ts         進度條
  src-tauri/
    Cargo.toml
    build.rs
    tauri.conf.json
    icons/                 由現有 icon.png 產生
    src/
      main.rs              Tauri 外殼
      patch/mod.rs         共用：backup_file / patch_binary / 進度事件
      patch/rich2.rs       RUN.EXE 的 4 條特徵碼
  build.ps1                改為驅動 tauri build
  main.py                  ← 暫時保留（oracle）
  rich2_patch.spec         ← 暫時保留
```

`RICH3_PATCH` 相同，但 `src-tauri/src/patch/` 多出 `calendar.rs`、`mkf.rs`、`rich3.rs`，且 `EVENTVOC`/`NEWSVOC`/`SCREEN` 改用 Tauri bundle resources。

### 共用規格（兩專案逐字元一致）

- `Cargo.toml` 的 release profile 沿用 editor：`opt-level = "s"`、`lto = true`、`codegen-units = 1`、`panic = "abort"`、`strip = true`
- `tauri.conf.json`：identifier `tw.overmind.rich2patch` / `tw.overmind.rich3patch`，bundle target 走 NSIS + portable
- 視窗尺寸統一，標題沿用 `大富翁2 Patch` / `大富翁3 Patch`
- 日誌等級字串沿用現行的 `INFO` / `WARN` / `ERROR` / `SUCCESS` / `FATAL` / `DONE`

---

## 5. 執行步驟

順序刻意讓 RICH2（只有 4 條特徵碼、無資源、無日曆）先跑通整條管線，確認架構沒問題再套到複雜的 RICH3。

### 步驟 1 — RICH2_PATCH Tauri 骨架
建立 Vite + TS 前端與 `src-tauri`，沿用 editor 的 Tauri 2 慣例。由現有 `icon.png` 產生 `src-tauri/icons/` 全套。
**交付：** 能啟動的空殼視窗。

### 步驟 2 — Rust 實作 RICH2 patch 引擎
移植 `main.py` 的 `backup_file` / `patch_binary` / `patch_exe`，包成 Tauri command。含 4 條 `RUN.EXE` 特徵碼（磁片版 ×2、光碟版 ×2）、`.bak` 備份策略（已存在則不覆蓋）、逐步驟日誌事件。
**交付：** 對真實 `RUN.EXE` 能跑出與 Python 版相同的結果。

### 步驟 3 — 系列共用前端 UI
遊戲目錄選擇（`plugin-dialog`）、開始按鈕、進度條、日誌區。視覺語言對齊 `RICH2_EDITOR`。
**交付：** 完整可操作的 RICH2 patcher。

### 步驟 4 — RICH3_PATCH 移植
複製骨架與 UI，移植全部 6 個步驟：
1. 內建資源釋放 — `EVENTVOC`/`NEWSVOC`/`SCREEN` 改為 Tauri resource，`resolve_resource` 取路徑
2. 日曆產生 — 14612 天的 `Cald.a` / `Cald.b`
3. EXE 特徵碼 — 14 條（含 1 條 regex 型的 CALD.A 搜尋組數）
4. `MAP.MKF` — 2 條物價修正
5. `SCREEN.MKF` — 索引表拆解與重組
6. 語音 MKF — `NEWSVOC` / `EVENTVOC` 注入

### 步驟 5 — 農曆換算一致性驗證（**關鍵，不可略過**）
Python 版用 `lunar_python`；Rust 端候選是 `lunar-rs 1.0.0-rc1`（基於壽星天文曆）。**兩者是不同實作，不保證輸出相同。**

驗證方式：用保留的 Python 版產生 `Cald.a` / `Cald.b` 當 oracle，與 Rust 版產出的 14612 天資料做 byte-for-byte 比對。

- 完全一致 → 放行
- 有差異 → 改用其他 crate，或自行移植 `lunar_python` 的換算表；**絕不接受「差幾天而已」**，這會直接讓遊戲內農曆顯示錯誤

### 步驟 6 — 改寫 `build.ps1` 並產出兩支 EXE
驅動 `npm ci` + `tauri build`，產出 portable exe 與 NSIS installer，沿用簽章流程，最後回報體積。實機啟動驗證。

產物命名必須符合 `RELEASE_RULES.md` 的保留中括號格式：

```text
[RICH2_PATCH][v1.0.0][Setup].exe
[RICH2_PATCH][v1.0.0][Portable].zip
```

---

## 6. 風險與待確認事項

### 🔴 WebView2 依賴（需要你決定）
Tauri 靠系統的 WebView2。Windows 10/11 內建，但 **Windows 7/8 沒有**，需要另外安裝執行期。

現行 tkinter 版是完全自足的單檔，在任何 Windows 上雙擊就能跑。考量到這是拿來修**DOS 時代老遊戲**的工具，使用者跑在老機器/老系統上的機率不低——這是換 Tauri 唯一真正的功能倒退。

> 若要保險，NSIS installer 可設定自動下載 WebView2 執行期（會讓安裝檔略大）；portable 版則無解。

### 🟡 農曆換算演算法差異
見步驟 5。這是整個遷移最可能卡關的地方，也是保留 Python 版的主因。

### 🟡 特徵碼搬運
RICH2 有 4 條、RICH3 有 14 條 EXE 特徵碼加 2 條 MAP 特徵碼，全部是手寫 hex。搬到 Rust 時逐條核對，並保留原本的中文名稱字串，方便和 Python 版的日誌對照。

### 🟡 MKF 重組邏輯
`SCREEN.MKF` 與語音 MKF 的索引表拆解/重建是自訂格式處理。Rust 版寫完後，應對同一份原始檔跑 Python 版與 Rust 版，比對輸出檔的 SHA-256。

### 🟢 簽章
現行走自簽 `Overmind.pfx` + signtool。Tauri 的 NSIS bundle 有內建簽章設定，可直接沿用同一張憑證。

---

## 7. 驗收標準

1. 兩支 portable exe 各自 **< 5 MB**
2. 對同一份遊戲原始檔，Tauri 版與 Python 版產出的所有檔案 **SHA-256 完全相同**（含 `Cald.a`/`Cald.b`/`RUN.EXE`/`RICH3.EXE`/`MAP.MKF`/`SCREEN.MKF`/兩個語音 MKF）
3. 兩支程式的 UI 版型、日誌格式、錯誤處理逐項對齊，看得出是同一系列
4. `build.ps1` 一鍵產出，兩專案指令一致
5. 驗收通過後才移除 `main.py` / `.spec` / `requirements.txt`

---

## 8. 發佈計畫

Tauri 版驗證通過後，三個專案統一發佈 **v1.0.0**，直接取代現有 release：

| Repo | 現有 tag | 現有狀態 | 處置 |
| :--- | :--- | :--- | :--- |
| `RICH2_PATCH` | `Rich2_Patch_v1.0` | Pre-release | 刪除，改發 `v1.0.0` |
| `RICH3_PATCH` | `Rich3_Patch_v1.0` | Pre-release | 刪除，改發 `v1.0.0` |
| `RICH2_EDITOR` | `v1.0.0` | Latest | 刪除，重發 `v1.0.0` |

舊的兩個 patch tag 本來就不符合 `RELEASE_RULES.md` 訂的命名慣例，這次一併清掉。
產物命名與 Release 頁面格式一律依 `RELEASE_RULES.md`。

`CHANGELOG.md` 三個專案目前都沒有，需補建（`RELEASE_RULES.md` 的發佈門檻與
Release 頁面內文都依賴它）。第一版直接寫目前已完成的功能。

---

## 9. 尚待討論的細節

1. **WebView2 的取捨**：接受 Win7/8 使用者需額外安裝執行期嗎？還是要保留 Python 版當作舊系統的備援發佈？（見第 6 節 🔴）
2. **發佈形式**：portable 單檔、NSIS 安裝檔，還是兩種都出？（patcher 通常是丟進遊戲資料夾就跑，portable 較合用）
