// 大富翁3 Patch —— Tauri 外殼與指令。
//
// 與 RICH2_EDITOR 刻意相反：patch 引擎放在 Rust（`patch` 模組），前端只負責畫面與事件。
// patcher 沒有瀏覽器版的需求，把二進位處理留在 Rust 就不必把檔案系統權限開放給前端，
// 前端只需要 dialog 來讓使用者挑資料夾（見 capabilities/default.json）。

pub mod patch;

use std::path::Path;

use chrono::Local;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use patch::{rich3, Reporter, FATAL};

/// 送到前端的日誌事件。
#[derive(Clone, Serialize)]
struct LogPayload {
    level: String,
    message: String,
    /// 有值代表這是一個步驟的開始，前端應同時推進進度條
    step: Option<u32>,
    total: u32,
}

/// 把引擎的輸出轉成 Tauri 事件。
struct TauriReporter {
    app: AppHandle,
}

impl Reporter for TauriReporter {
    fn log(&self, message: &str, level: &str, step: Option<u32>) {
        // 同時印到終端機，格式與 Python 版一致，方便開發時兩版並排對照
        let tag = match step {
            Some(s) => format!("[STEP {s}/{}]", rich3::TOTAL_STEPS),
            None => "[DETAILS]".to_string(),
        };
        println!("[{level}]{tag} {message}");

        let _ = self.app.emit(
            "patch://log",
            LogPayload {
                level: level.to_string(),
                message: message.to_string(),
                step,
                total: rich3::TOTAL_STEPS,
            },
        );
    }
}

/// 對指定的遊戲目錄執行全套 patch。
///
/// 一定要放在背景執行緒：光是產生 14612 天的日曆就要數十秒，跑在 UI 執行緒上會整個卡住。
#[tauri::command]
async fn run_patch(app: AppHandle, target_dir: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let reporter = TauriReporter { app };
        // 日曆以「執行當下」為基準，這裡是唯一取系統時間的地方
        let today = Local::now().date_naive();
        match rich3::run_patch(Path::new(&target_dir), today, &reporter) {
            Ok(summary) => Ok(summary),
            Err(err) => {
                let message = format!("幹，Patch 發生嚴重錯誤：\n{err}");
                reporter.log(&message, FATAL, None);
                Err(message)
            }
        }
    })
    .await
    .map_err(|err| format!("背景執行緒異常結束：{err}"))?
}

/// 程式啟動時的預設目錄。
///
/// 沿用 Python 版的貼心設計：預設帶入程式所在目錄，讓「把程式丟進遊戲資料夾直接執行」
/// 也能用。放在 Rust 端做是刻意的——前端因此不需要任何檔案系統權限。
#[tauri::command]
fn default_dir() -> String {
    std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![run_patch, default_dir])
        .run(tauri::generate_context!())
        .expect("Tauri 啟動失敗");
}
