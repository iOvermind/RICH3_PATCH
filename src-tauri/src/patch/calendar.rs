// 日曆檔 Cald.a / Cald.b 的產生。
//
// 原版遊戲的日曆是固定資料，2018/6/6 之後農曆就會崩潰。這裡改成以**執行當下**為基準
// 動態產生：往前 10 年、共 14612 天（40 年）。天數會回傳給 EXE patch 寫進搜尋組數。
//
// 兩個檔案的格式相同，每天 4 個位元組：
//   [0] 日  [1] 月  [2..4] 年（16 位小端）
// Cald.a 放國曆，Cald.b 放農曆。農曆的閏月在函式庫中以負數表示，取絕對值後寫入——
// 這一點與 Python 版的 `abs(lunar.getMonth())` 相同。
//
// ⚠ 農曆換算的正確性是整個 Tauri 版的關鍵。這裡用的 `lunar-rs` 是 6tail
// `lunar-javascript` / `lunar-go` 的移植，與 Python 版的 `lunar_python` 同源，
// 已實測 14612 天逐位元組相同（見 DEVELOPER.md §6）。換掉這個相依前必須重跑比對。

use chrono::{Datelike, NaiveDate};
use lunar_rs::Solar;
use std::fs;
use std::io;
use std::path::Path;

use super::Reporter;

/// 涵蓋天數。40 年，與 Python 版相同。
pub const TOTAL_DAYS: usize = 14612;

/// 起始年份的偏移：以當年往前推 10 年為起點。
const START_YEAR_OFFSET: i32 = 10;

fn encode(day: u32, month: u32, year: i32, out: &mut Vec<u8>) {
    out.push(day as u8);
    out.push(month as u8);
    out.extend_from_slice(&(year as u16).to_le_bytes());
}

/// 產生 `Cald.a` 與 `Cald.b`，回傳實際寫入的天數。
///
/// `today` 由呼叫端傳入而非在這裡取系統時間——這樣測試與 oracle 比對才能固定基準日期。
pub fn generate(
    target_dir: &Path,
    today: NaiveDate,
    reporter: &dyn Reporter,
    step: u32,
) -> io::Result<usize> {
    reporter.log(
        "開始產生動態滑動日曆 Cald.a 與 Cald.b...",
        super::INFO,
        Some(step),
    );

    // 先備份原始日曆檔
    for name in ["Cald.a", "Cald.b"] {
        let path = target_dir.join(name);
        if path.exists() {
            let mut bak = path.as_os_str().to_os_string();
            bak.push(".bak");
            let bak = std::path::PathBuf::from(bak);
            if !bak.exists() {
                fs::copy(&path, &bak)?;
                reporter.info(&format!(
                    "📦 已安全備份原檔: {} -> {}",
                    path.display(),
                    bak.display()
                ));
            }
        }
    }

    let start = NaiveDate::from_ymd_opt(today.year() - START_YEAR_OFFSET, 1, 1)
        .expect("起始日期不合法");
    let end = start + chrono::Duration::days(TOTAL_DAYS as i64 - 1);
    reporter.info(&format!(
        "涵蓋範圍：{start} 至 {end} (共 {TOTAL_DAYS} 天)"
    ));

    let mut cald_a = Vec::with_capacity(TOTAL_DAYS * 4);
    let mut cald_b = Vec::with_capacity(TOTAL_DAYS * 4);
    let mut date = start;

    for _ in 0..TOTAL_DAYS {
        encode(date.day(), date.month(), date.year(), &mut cald_a);

        let lunar = Solar::from_ymd(date.year(), date.month() as i32, date.day() as i32)
            .expect("國曆日期不合法")
            .lunar();
        encode(
            lunar.get_day() as u32,
            lunar.get_month().unsigned_abs(),
            lunar.get_year(),
            &mut cald_b,
        );

        date = date.succ_opt().expect("日期溢位");
    }

    fs::write(target_dir.join("Cald.a"), &cald_a)?;
    fs::write(target_dir.join("Cald.b"), &cald_b)?;

    reporter.info("日曆產生完畢！完美瘦身避免 Buffer Overflow。");
    Ok(TOTAL_DAYS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Silent;
    impl Reporter for Silent {
        fn log(&self, _message: &str, _level: &str, _step: Option<u32>) {}
    }

    struct Collector(RefCell<Vec<String>>);
    impl Reporter for Collector {
        fn log(&self, message: &str, _level: &str, _step: Option<u32>) {
            self.0.borrow_mut().push(message.to_string());
        }
    }

    #[test]
    fn 產出長度為天數乘四() {
        let dir = std::env::temp_dir().join(format!("rich3_cal_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
        let days = generate(&dir, today, &Silent, 2).unwrap();

        assert_eq!(days, TOTAL_DAYS);
        assert_eq!(fs::metadata(dir.join("Cald.a")).unwrap().len() as usize, TOTAL_DAYS * 4);
        assert_eq!(fs::metadata(dir.join("Cald.b")).unwrap().len() as usize, TOTAL_DAYS * 4);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn 起點為當年往前十年的元旦() {
        let dir = std::env::temp_dir().join(format!("rich3_cal_start_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let reporter = Collector(RefCell::new(Vec::new()));
        generate(&dir, NaiveDate::from_ymd_opt(2026, 8, 6).unwrap(), &reporter, 2).unwrap();

        let messages = reporter.0.borrow().clone();
        assert!(
            messages.iter().any(|m| m.contains("2016-01-01") && m.contains("2056-01-02")),
            "涵蓋範圍訊息不如預期：{messages:?}"
        );

        // 第一天必須是 2016-01-01
        let a = fs::read(dir.join("Cald.a")).unwrap();
        assert_eq!(a[0], 1, "日");
        assert_eq!(a[1], 1, "月");
        assert_eq!(u16::from_le_bytes([a[2], a[3]]), 2016, "年");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn 農曆閏月以絕對值寫入() {
        // 2020 年閏四月：2020-06-01 是閏四月初十
        let lunar = Solar::from_ymd(2020, 6, 1).unwrap().lunar();
        assert!(lunar.get_month() < 0, "閏月在函式庫中應為負數");
        assert_eq!(lunar.get_month().unsigned_abs(), 4);
    }

    #[test]
    fn 既有的_bak_不會被覆蓋() {
        let dir = std::env::temp_dir().join(format!("rich3_cal_bak_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("Cald.a"), b"original-a").unwrap();
        fs::write(dir.join("Cald.b"), b"original-b").unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
        generate(&dir, today, &Silent, 2).unwrap();
        assert_eq!(fs::read(dir.join("Cald.a.bak")).unwrap(), b"original-a");

        // 再跑一次，.bak 必須維持最原始的內容
        generate(&dir, today, &Silent, 2).unwrap();
        assert_eq!(fs::read(dir.join("Cald.a.bak")).unwrap(), b"original-a");

        fs::remove_dir_all(&dir).unwrap();
    }
}
