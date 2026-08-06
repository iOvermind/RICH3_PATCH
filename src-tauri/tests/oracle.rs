//! 拿真實遊戲檔跑一遍 Rust 引擎，供與 Python 版做逐位元組比對。
//!
//! 遊戲原始檔**不進版控**（版權），所以這個測試預設會被略過而不是失敗——與
//! RICH2_EDITOR 的測試慣例一致。
//!
//! 基準日期固定為 2016-01-01 起算（相當於在 2026 年執行），Python 版必須用同一個
//! 基準跑，否則日曆內容一定不同——`generate` 之所以把 `today` 當參數而非在內部取
//! 系統時間，就是為了這件事。
//!
//! 用法：
//! ```powershell
//! $env:RICH3_GAME_DIR = 'D:\path\to\three'    # 未修改的遊戲目錄，會被複製，不會被改到
//! $env:RICH3_OUT_DIR  = 'D:\path\to\out'      # 產出位置，供外部比對雜湊
//! cargo test --test oracle -- --nocapture
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use rich3_patch::patch::{calendar, rich3, Reporter};

struct StdoutReporter;

impl Reporter for StdoutReporter {
    fn log(&self, message: &str, level: &str, step: Option<u32>) {
        let tag = match step {
            Some(s) => format!("[STEP {s}/{}]", rich3::TOTAL_STEPS),
            None => "[DETAILS]".to_string(),
        };
        println!("[{level}]{tag} {message}");
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[test]
fn 對真實遊戲檔執行() {
    let Ok(game_dir) = std::env::var("RICH3_GAME_DIR") else {
        println!("⏭ 略過：未設定 RICH3_GAME_DIR（遊戲原始檔不進版控，這不算失敗）");
        return;
    };

    let out_dir = std::env::var("RICH3_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("rich3_patch_oracle"));

    let _ = fs::remove_dir_all(&out_dir);
    copy_dir(Path::new(&game_dir), &out_dir).expect("複製遊戲目錄失敗");

    // 與 Python 版對比時必須用同一個基準日期
    let today = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
    let summary = rich3::run_patch(&out_dir, today, &StdoutReporter).expect("執行失敗");
    println!("\n{summary}");

    // 日曆長度是可獨立驗證的硬指標
    for name in ["Cald.a", "Cald.b"] {
        let len = fs::metadata(out_dir.join(name)).unwrap().len() as usize;
        assert_eq!(len, calendar::TOTAL_DAYS * 4, "{name} 長度應為天數 × 4");
    }

    // 備份必須等於原始檔
    for name in ["RICH3.EXE", "MAP.MKF", "SCREEN.MKF", "NEWSVOC.MKF", "EVENTVOC.MKF"] {
        let bak = out_dir.join(format!("{name}.bak"));
        if !bak.exists() {
            continue; // 該版本沒有這個檔就跳過
        }
        let original = Path::new(&game_dir).join(name);
        if original.exists() {
            assert_eq!(
                fs::read(&bak).unwrap(),
                fs::read(&original).unwrap(),
                "{name}.bak 必須逐位元組等於原始檔"
            );
        }
    }

    // 步驟 1 建立的暫存資源資料夾必須被清掉
    for folder in ["EVENTVOC", "NEWSVOC", "SCREEN"] {
        let existed_before = Path::new(&game_dir).join(folder).exists();
        let exists_after = out_dir.join(folder).exists();
        assert_eq!(
            exists_after, existed_before,
            "{folder}：原本不存在就該被清除，原本存在就該保留"
        );
    }

    println!("\n產出位置：{}", out_dir.display());
}
