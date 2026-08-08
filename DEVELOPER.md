# 大富翁3 Patch 開發者文件

> 這份文件給**要修改這個專案的人**。使用說明請看 [README.md](README.md)。
> 文件與發佈規範見 [docs/rules/](docs/rules/)。

> **遷移已完成。** `main` 是 Rust + Tauri，發佈中的版本也是它。Python + tkinter 版
> 已從 `main` 移除，完整保留在 `legacy/python-tkinter` 分支（見 §8）。
> 過程與驗收記錄在 [TAURI_MIGRATION.md](TAURI_MIGRATION.md)。

---

## 1. 技術棧與系統需求

### Tauri 版（`main` 的主線）

| 項目 | 版本 | 用途 |
| :--- | :--- | :--- |
| Rust | 1.77 以上（實測 1.97.1） | patch 引擎與桌面外殼 |
| Tauri | 2 | 桌面框架 |
| `lunar-rs` | 1.0.0-rc1 | 農曆換算 |
| `chrono` | 0.4 | 國曆日期迭代 |
| Node.js | 實測 v25.8.1 | 前端建置 |
| TypeScript / Vite / Tailwind | 5.9 / 8 / 4 | 前端 |
| MSVC Build Tools | — | 編譯 Rust，需「使用 C++ 的桌面開發」工作負載 |
| WebView2 Runtime | — | **執行**所需，Windows 10/11 已內建 |

**作業系統限制**：產物僅供 Windows。

要重跑 oracle 比對時才需要 Python 3.10 以上與 `lunar_python`（`main` 上已無 Python 檔案，見 §6）。

---

## 2. 環境建置

1. 取得原始碼
   ```bash
   git clone git@github.com:iOvermind/RICH3_PATCH.git
   ```

2. 安裝前端相依
   ```powershell
   npm ci
   ```

3. 安裝 Rust 工具鏈
   ```powershell
   winget install Rustlang.Rustup
   ```
   再從 Visual Studio Installer 安裝「**使用 C++ 的桌面開發**」工作負載。

4. 驗證
   ```powershell
   npm run build
   cd src-tauri; cargo test
   ```

---

## 3. 日常開發

```powershell
npm run app:dev            # 桌面版：原生視窗 + Vite HMR
npm run dev                # 只開前端，沒有 Tauri API
```

**除錯**：引擎的每一則訊息同時 `println!` 到終端機（格式與 Python 版逐字相同，方便並排對照）與發出 Tauri 事件 `patch://log`，酬載為 `{ level, message, step, total }`。

狀態字串為 `INFO` / `WARN` / `ERROR` / `SUCCESS` / `FATAL` / `DONE`，Rich Patch Series 共用。

⚠ **日曆那一步要跑數十秒。** 只想測其他步驟時可以暫時把 `rich3::run_patch` 裡的 `calendar::generate` 換成固定天數，但**別忘了改回來**——第 3 步的特徵碼依賴它的回傳值。

---

## 4. 目錄結構

```text
RICH3_PATCH/
├─ index.html               視窗外殼（版面與 RICH2_PATCH 逐字相同，只有文案不同）
├─ package.json             版本號的單一來源
├─ src/
│  ├─ main.ts               前端進入點（與 RICH2_PATCH 逐字相同）
│  └─ style.css             Tailwind 設定；色票引用 docs/rules/tokens.css
├─ EVENTVOC/ NEWSVOC/ SCREEN/   自製資源，編譯期嵌入執行檔（見 §5）
├─ src-tauri/
│  ├─ build.rs              產生嵌入資源的程式碼 + tauri_build
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ capabilities/         Tauri 權限宣告（見 §9.2）
│  ├─ src/
│  │  ├─ main.rs            進入點，只呼叫 lib
│  │  ├─ lib.rs             Tauri 外殼、指令、事件轉送
│  │  └─ patch/
│  │     ├─ mod.rs          模組宣告
│  │     ├─ engine.rs       ⚠ 共用引擎，與 RICH2_PATCH **逐字元相同**
│  │     ├─ calendar.rs     Cald.a / Cald.b 產生
│  │     ├─ mkf.rs          MKF 封裝檔的拆解與重組
│  │     └─ rich3.rs        六個步驟、EXE 的 14 條與 MAP.MKF 的 7 條特徵碼
│  └─ tests/oracle.rs       拿真實遊戲檔跑一遍，供與 Python 版比對
├─ build.ps1                建置與發佈打包（含版本號一致性檢查）
└─ docs/rules/              文件與發佈規範（正典在 DEV_TEMPLATE）
```

---

## 5. 架構與關鍵設計決策

### 六個步驟

`rich3::run_patch` 依序執行，`TOTAL_STEPS = 6`：

| 步驟 | 函式 | 做什麼 |
| :-: | :--- | :--- |
| 1 | `extract_bundled_folders` | 把嵌入的 `EVENTVOC` / `NEWSVOC` / `SCREEN` 釋放到遊戲目錄 |
| 2 | `calendar::generate` | 產生 `Cald.a`（國曆）與 `Cald.b`（農曆），共 14612 天 |
| 3 | `patch_exe` | `RICH3.EXE` 的 14 條特徵碼（含 1 條帶萬用位元組） |
| 4 | `patch_map_mkf` | `MAP.MKF` 的 7 條修正（2 條物價、5 條地點資料誤植） |
| 5 | `patch_screen_mkf` | `SCREEN.MKF` 的索引表拆解與重組 |
| 6 | `patch_audio_mkf` | `NEWSVOC` 與 `EVENTVOC` 兩個語音 MKF 的注入 |

### 關鍵決策

#### 內建資源用 `include_bytes!` 嵌入，不用 Tauri 的 bundle resources

- **決定**：`build.rs` 走訪三個資源資料夾，產生一張 `(資料夾, 檔名, 內容)` 的靜態表，在編譯期把約 440 KB 的資源嵌進執行檔。
- **理由**：**Tauri 的 `bundle.resources` 是把檔案放在安裝目錄旁邊，只對安裝版有效**；我們的主力產物是 portable 單檔 exe，那樣拿不到資源。Python 版靠 PyInstaller onefile 達成同樣效果。（遷移計畫書原本寫「改用 Tauri bundle resources」，實作時發現行不通。）
- **代價**：資源變動要重新編譯（`build.rs` 已設 `rerun-if-changed`），且執行檔多了 440 KB。

#### 日曆的「今天」由呼叫端傳入

- **決定**：`calendar::generate(target_dir, today, ...)` 不在內部取系統時間；只有 `lib.rs` 的指令會呼叫 `Local::now()`。
- **理由**：**這是 oracle 比對能成立的前提**。日曆內容依執行日期而變，兩版若不是用同一個基準日期跑，結果一定不同，比對就失去意義。
- **代價**：多一個參數。

#### 農曆相依選 `lunar-rs`

- **決定**：用 `lunar-rs`，而非自行移植換算表或選其他 crate。
- **理由**：它是 6tail `lunar-javascript` / `lunar-go` 的移植，與 Python 版的 `lunar_python` **同源**。實測 14612 天的 `Cald.a` / `Cald.b` 與 Python 版**逐位元組相同**。
- **代價**：綁在一個 `1.0.0-rc1` 的相依上。**換掉或升級它之前必須重跑 §6 的比對**——農曆錯一天，遊戲裡的節日就全錯。

#### 替換範圍由呼叫端明示

- **決定**：EXE 用 `ReplaceMode::First`，`MAP.MKF` 用 `ReplaceMode::All`。
- **理由**：EXE 的特徵碼是特定指令位置，全域替換可能誤傷；資料檔的同一筆數值可能合法地出現多次且都該改。Python 版是用副檔名判斷，這裡改成明示。
- **代價**：新增目標檔時要自己想清楚該用哪一種。

#### 地點資料的誤植用特徵碼歸零，不做結構化改寫

- **決定**：`MAP.MKF` 的三處原版誤植（v1.0.2 加入）與物價修正走同一套特徵碼機制，把誤填的欄位歸零，不去解析 `#02` 表再重寫。
- **理由**：引擎已經有的東西就夠用。每條特徵碼都在整份 `MAP.MKF`（1,944,084 byte）裡驗證過**只出現一次**，所以用 `ReplaceMode::All` 也不會誤傷。
- **代價**：特徵碼綁死在這一份原版資料上；換了來源檔就得重新算。

看懂這些數字需要知道 `#02 座標地點轉換表`的格式——每張地圖 50×50 格、**每格 10 byte**，五個小端 u16，格子索引為 `x + y * 50`：

| 位移 | 欄位 |
| :--- | :--- |
| 0–1 | 路段編號（**名稱由此而來**：名稱索引 = 路段編號 + 0x54，去 `WORD.DAT` 查） |
| 2–3 | 地點形態（0=無 1=事件 2=道路 3=港口 4=車站 5=海港 6=機場 7=建地 8=租地 9=公司） |
| 4–5 | 實際作用地點 X |
| 6–7 | 實際作用地點 Y |
| 8–9 | 資訊編號（意義依形態而定） |

三處誤植與各自的位移（**改完只該有 13 個 byte 不同**，`patch/rich3.rs` 的單元測試會擋）：

| 誤植 | 症狀 | 位移 | 動幾個 byte |
| :--- | :--- | :--- | :-: |
| 大陸 (1,0) | 路段 39、形態 33，顯示成「中山南路」。形態 33 超出 0–9，是全部 7,500 格裡唯一一個 | `0x1D1C1E` | 4 |
| 台北 (29,30) (29,31) (29,32) | 建地 81/82/83 的建築物圖帶了路段 224，顯示成「新生北路」 | `0x7DF36` / `0x7E12A` / `0x7E31E` | 各 1 |
| 大陸 (46,36) (47,36) | 被誤填成形態 1／路段 12／資訊 249，害公園變成 2×3。這兩格用的方塊是 481/482，根本不是公園的圖（真公園四格用 0/1/2/3） | `0x1D6430` | 6 |

兩個下手時容易寫壞的地方：

- **台北那三格只清前 2 byte 的路段編號**，`0x51`/`0x52`/`0x53` 的資訊編號要留著——那是這三格與地產的關聯，動了就會影響買賣與收租。
- **大陸公園那兩格相鄰，必須寫成一條規則**。拆成兩條的話，第一條改完之後第二條的特徵碼就對不上了。該條前後各帶一格當上下文，靠它們湊出唯一的特徵碼。

#### 只清除自己建立的資源資料夾

- **決定**：步驟 1 記錄「本次新建」的資料夾，結束時只刪這些；原本就存在的只覆寫內容，不刪除。
- **理由**：不刪使用者原本就有的東西。
- **代價**：使用者若自己建過同名資料夾，跑完會留在遊戲目錄裡。

#### 只跟 RICH2_EDITOR 共用色票與字體，版型完全自己來

- **決定**：`src/style.css` 的 `@theme` 引用 `docs/rules/tokens.css` 的共用 token，版型則是單欄小工具，與 RICH2_PATCH 逐字相同。
- **理由**：editor 是 1500×950 的編輯工作站，patcher 是按一下就跑完的小工具。共用色票已足以看出是同一系列。
- **代價**：畫面程式碼無法與 editor 互通，只有設計 token 共用。

---

## 6. 測試

```powershell
cd src-tauri
cargo test
```

| 分類 | 涵蓋範圍 |
| :--- | :--- |
| `patch/engine.rs` | 十六進位解析（含萬用位元組）、兩種替換模式、備份不覆蓋、沒命中就不寫檔 |
| `patch/mkf.rs` | 拆解／重組往返一致、區塊長度改變後索引表重算、空檔案不崩潰 |
| `patch/calendar.rs` | 產出長度、起點為當年往前十年的元旦、閏月取絕對值、`.bak` 不覆蓋 |
| `patch/rich3.rs` | 21 條特徵碼長度一致、地點誤植修正的位元組數、天數寫進搜尋組數、序號解析、嵌入資源數量 |
| `tests/oracle.rs` | 拿真實遊戲檔跑完整流程 |

### 會被略過的測試

`tests/oracle.rs` 需要真實遊戲檔，而**遊戲原始檔不進版控**（版權）。未設環境變數時會印出 `⏭ 略過` 並通過——**這代表沒測到，不代表測過了**。

```powershell
$env:RICH3_GAME_DIR = '<未修改的遊戲目錄>'   # 目前放在 RICH3_EDITOR\rich3\original\
$env:RICH3_OUT_DIR  = '<產出位置>'
cargo test --test oracle -- --nocapture
```

### 與 Python 版的 oracle 比對

這是 Tauri 版能否取代 Python 版的判準，**農曆是其中風險最高的一項**。動到 `patch/` 或 `lunar-rs` 版本後**必須**重跑。

Python 版已從 `main` 移除，要跑基準時從分支取出到庫外：

```powershell
git worktree add ..\rich3-oracle legacy/python-tkinter
python -m pip install -r ..\rich3-oracle\requirements.txt
```

⚠ **兩邊必須用同一個基準日期。** 日曆以「執行當下年份 −10」為起點，不固定日期就一定對不上。Rust 端由 `tests/oracle.rs` 指定；Python 端要覆寫 `datetime.datetime.now()`：

```python
import sys, datetime
sys.path.insert(0, r'..\rich3-oracle')
class Fixed(datetime.datetime):
    @classmethod
    def now(cls, tz=None): return cls(2026, 8, 7, 12, 0, 0)   # 與 Rust 端同一天
import main
main.datetime.datetime = Fixed
main.run_patch(r'<py 複本>', lambda m: None, lambda m: sys.exit(1))
```

⚠ 未打包的 Python 偵測不到 `_MEIPASS`，會跳過資源釋放——要先把 `EVENTVOC` / `NEWSVOC` / `SCREEN` 複製進 py 複本，打包版才會自己釋放。

比對範圍：`Cald.a`、`Cald.b`、`RICH3.EXE`、`SCREEN.MKF`、`NEWSVOC.MKF`、`EVENTVOC.MKF` 的 SHA-256 全部相同。

⚠ **`MAP.MKF` 自 v1.0.2 起不再與 Python 版相同**，因為 v1.0.2 新增了 5 條 Python 版沒有的地點資料誤植修正。`legacy/python-tkinter` 不會補上這些規則（該分支只是農曆的比對基準）。要比對 `MAP.MKF` 時改用這個判準：**Rust 版產出與自己的 `MAP.MKF.bak` 相差 25 個位元組**——12 個來自兩條物價修正（各命中 3 處），13 個來自地點誤植修正（大陸 (1,0) 四個、台北三格各一個、大陸公園兩格各三個）。位元組數由 `patch/rich3.rs` 的單元測試把關，實際偏移見 §5「地點資料的誤植用特徵碼歸零」。

### 兩個層級都要驗

| 層級 | 驗什麼 | 怎麼跑 |
| :--- | :--- | :--- |
| 函式庫 | 引擎邏輯正確 | `cargo test --test oracle` |
| **發佈產物** | **打包、LTO、strip、資源嵌入之後行為仍相同** | 直接執行 `release\` 裡的 portable exe |

只驗函式庫是不夠的——中間隔著 Tauri 打包與資源嵌入。產物層級要用 GUI 跑，關鍵在**取得前景視窗**：

```powershell
# Windows 不允許背景程序隨意搶前景，光呼叫 SetForegroundWindow 會被擋。
# 必須先 AttachThreadInput 把自己的輸入佇列接到目標與現有前景視窗的執行緒上。
AttachThreadInput(me, foreThread, true);
AttachThreadInput(me, targetThread, true);
ShowWindow(h, SW_RESTORE); BringWindowToTop(h); SetForegroundWindow(h);
```

沒做這步的話模擬點擊約有一半機率靜靜地不生效，看起來像程式沒反應。

**2026-08-07 實測**：以 `rich3\original` 為來源、基準日期 2026-08-07，實際發佈的
`RICH3_PATCH-v1.0.1-Portable.exe` 與 Python 版產出的**七個檔案全數逐位元組相同**。

**2026-08-08 實測**：v1.0.2 加入地點誤植修正後，兩個層級都重跑過。

- **函式庫層級**（`cargo test --test oracle`，以 `rich3\original` 為來源、基準日期 2026-08-06）：
  `MAP.MKF` 以外的六個檔案與 Python 版逐位元組相同；`MAP.MKF` 與 Python 版只差新增的那
  13 個位元組，偏移與 §5 的對照表逐項一致。
- **產物層級**：實際發佈的 `RICH3_PATCH-v1.0.2-Portable.exe` 放進遊戲目錄複本執行，
  `RICH3.EXE`、`MAP.MKF`、`SCREEN.MKF`、`NEWSVOC.MKF`、`EVENTVOC.MKF` 與函式庫層級的產出
  **逐位元組相同**，五個 `.bak` 等於未修改的原始檔，`Cald.a` / `Cald.b` 各 58448 byte，
  暫存資源資料夾已清除。

---

## 7. 建置與產物

```powershell
.\build.ps1                # 建置並收進 release\
.\build.ps1 -Sign          # 另外用自簽憑證簽章
.\build.ps1 -SkipInstall   # 跳過 npm ci
```

流程：版本號一致性檢查 → `npm ci` → `tauri build` → 依規範命名收進 `release\` → 選擇性簽章 → 產生 `SHA256SUMS.txt` → 回報體積。

**產物**

| 產物 | 用途 |
| :--- | :--- |
| `release/RICH3_PATCH-v1.0.2-Portable.exe` | 免安裝，直接執行 |
| `release/RICH3_PATCH-v1.0.2-Setup.exe` | NSIS 安裝檔 |
| `release/SHA256SUMS.txt` | 校驗碼，**必附**（RELEASE_RULES §4.3） |

命名依 [docs/rules/RELEASE_RULES.md](docs/rules/RELEASE_RULES.md) §2.1：只用 `A-Za-z0-9.-_`，因為 GitHub 會把其餘字元換成點。`release/` 不進版控。

### 版本號

**單一來源**：`package.json` 的 `version`

| 位置 | 欄位 | 方式 |
| :--- | :--- | :--- |
| `package.json` | `version` | 手動（單一來源） |
| `src-tauri/tauri.conf.json` | `version` | 手動 |
| `src-tauri/Cargo.toml` | `package.version` | 手動 |
| `src-tauri/Cargo.lock` | `rich3-patch` 的 `version` | 自動 |
| 產物檔名 | — | 自動（`build.ps1` 讀取單一來源） |

`build.ps1` 開頭會把前三處讀出來比對，**不一致就中止**。

---

## 8. 分支、commit 與 PR 慣例

- **主分支**：`main`
- **開分支**：從 `main` 開，功能用 `feat/<描述>`、修正用 `fix/<描述>`。
- **commit 訊息**：首行為繁中祈使句摘要，必要時空一行後補理由。

### 舊實作的保留

`main` 走 Rust + Tauri 版，Python + tkinter 版依 `docs/rules/DEVELOPER_RULES.md` §4.3 以分支保留：

| 分支 | 內容 | 保留原因 | 解除條件 |
| :--- | :--- | :--- | :--- |
| `legacy/python-tkinter` | 遷移前的完整 Python + tkinter 實作，含 `lunar_python` 的農曆換算與打包腳本 | 是驗證 Tauri 版正確性的基準（oracle）。農曆是最大風險——`lunar-rs` 與 `lunar_python` 是不同專案，必須逐位元組確認 | **無**——這個分支永久保留 |

**該分支不再接受新功能**，僅在有明確理由時接受修正。**不得刪除**。

---

## 9. 安全與敏感資料

### 9.1 機密不進版控

| 項目 | 排除方式 | 本機該放哪 |
| :--- | :--- | :--- |
| 簽章憑證 `Overmind.pfx` | `.gitignore` 的 `*.pfx` | 專案根目錄，由建置腳本自動產生 |
| 遊戲原始檔（`*.EXE` / `*.MKF` / `*.PAT` / `*.a` / `*.b` / `*.bak`…） | `.gitignore` 逐項排除 | 庫外，或已忽略的目錄 |

⚠ **憑證密碼 `overmind` 以明文寫在 `build.ps1` 中。** 該憑證為自簽、僅用於讓 EXE 帶上發行者名稱，不具信任價值，密碼公開不造成額外風險。**因此這張憑證不得用於任何其他用途**。

**注意**：`EVENTVOC/`、`NEWSVOC/`、`SCREEN/` 是本專案自製的資源（語音以 GPT-SoVITS 合成），**刻意進版控**；`.gitignore` 排除的 `*.MKF` 是遊戲原始檔，兩者不要搞混。

### 9.2 權限最小化

| 要求的權限 | 為什麼需要 |
| :--- | :--- |
| `core:default` | Tauri 基本功能 |
| `dialog:allow-open` | 讓使用者用系統對話框挑選遊戲資料夾 |

⚠ **前端刻意沒有任何 `fs` 權限。** 所有檔案讀寫都在 Rust 端完成，前端拿到的只是一個路徑字串。

程式不連網、不寫登錄檔、不需要管理員權限。

### 9.3 依賴來源與鎖檔

- `package-lock.json` 與 `src-tauri/Cargo.lock` **都進版控**。
- 安裝一律用 **`npm ci`**。
- **`lunar-rs` 的換算結果直接決定遊戲內農曆是否正確**，升版本後必須重跑 §6 的比對。

### 9.4 破壞性操作的保護

| 操作 | 影響的資料 | 可回復機制 |
| :--- | :--- | :--- |
| 改寫 `RICH3.EXE`、`MAP.MKF`、`SCREEN.MKF`、兩個語音 MKF | 使用者的遊戲檔案 | 先複製為 `<原檔名>.bak`；**若 `.bak` 已存在則不覆蓋** |
| 覆寫 `Cald.a` / `Cald.b` | 遊戲日曆檔 | 同上。日曆**每次執行都會重新產生**，這是刻意的 |
| 釋放資源資料夾 | `EVENTVOC` / `NEWSVOC` / `SCREEN` | 程式建立的會在結束時清除；**使用者原本就有的只覆寫、不刪除** |

---

## 10. 已知陷阱

#### 14 條 EXE 特徵碼不會全部命中

- **症狀**：日誌顯示 `已儲存修改 (11/14 項)`，其中「修正住院/坐牢免付過路費位置」、「破解顏色密碼 (磁片版)」、「破解光碟檢查 (相容項 1)」顯示跳過。
- **原因**：磁片版、重訂光碟版、Steam 典藏版的偏移位址不同，程式把各版本的特徵碼全部列出逐一嘗試。
- **處置**：正常行為。Steam 典藏版實測即為 **11/14**。看關鍵項目有沒有命中，不是看是否全中。

#### 日曆檔每次執行的內容都不同，無法用固定雜湊比對

- **症狀**：同一份遊戲檔跑兩次，`Cald.a` / `Cald.b` 的 SHA-256 不一樣。
- **原因**：日曆以執行當下年份 −10 為起點，換一天執行就是不同資料。這是功能不是錯誤。
- **處置**：做輸出比對時**必須固定基準日期**。`calendar::generate` 的 `today` 是參數就是為了這件事；`tests/oracle.rs` 固定為 2026-08-06，Python 版那邊也要改成同一天。

#### 改了農曆相依卻沒重跑比對

- **症狀**：單元測試全過，但遊戲裡的農曆日期或節日錯掉。
- **原因**：`lunar-rs` 與 `lunar_python` 是不同專案，只是同源。任何版本變動都可能讓某些年份的閏月判斷不同。
- **處置**：動到 `lunar-rs` 版本後**必須**重跑 §6 的 14612 天比對。這是 Python 版還留著的主要理由。

#### 資源改了卻沒生效

- **症狀**：換了 `EVENTVOC/` 裡的檔案，但跑出來還是舊的。
- **原因**：資源是**編譯期**嵌入的。`build.rs` 已設 `cargo:rerun-if-changed`，但若只改檔案內容而路徑不變，某些情況下 cargo 仍可能沿用快取。
- **處置**：確認有重新編譯；必要時 `cargo clean -p rich3-patch` 再建一次。

#### 編輯 `build.ps1` 後整支腳本變成語法錯誤

- **症狀**：一堆 `Unexpected token`，錯誤都指向含中文的行。
- **原因**：Windows PowerShell 5.1 讀 `.ps1` 預設用系統 ANSI 碼頁，無 BOM 的 UTF-8 中文會被拆壞。
- **處置**：`build.ps1` **必須**存成 **UTF-8 with BOM**。這是 `.ps1` 專屬例外——Markdown 一律無 BOM。

#### `npm` 或 `cargo` 明明跑成功，腳本卻中止

- **症狀**：`build.ps1` 報 `NativeCommandError`，內容卻是正常的進度訊息。
- **原因**：PowerShell 5.1 把原生指令的 stderr 每一行包成 `ErrorRecord`，配上 `$ErrorActionPreference = 'Stop'` 就會中止。
- **處置**：原生指令一律走 `build.ps1` 的 `Invoke-Native`，成敗只看離開碼。

---

## 相關文件

- 使用說明：[README.md](README.md)
- 介面規格：[INTERFACE.md](INTERFACE.md)
- 變更紀錄：[CHANGELOG.md](CHANGELOG.md)
- 遷移計畫：[TAURI_MIGRATION.md](TAURI_MIGRATION.md)
- 文件與發佈規範：[docs/rules/](docs/rules/)
