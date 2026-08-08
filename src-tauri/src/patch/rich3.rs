// 《大富翁3》專屬的六個 patch 步驟。
//
// 特徵碼與流程逐條移植自 Python 版 `main.py`，**中文名稱字串刻意保持一字不差**，
// 兩版的日誌可以直接並排對照（見 TAURI_MIGRATION.md 步驟 4 的驗收方式）。

use chrono::NaiveDate;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    backup_file, calendar, find_target, format_report, mkf, patch_binary, ReplaceMode, Reporter,
    Rule, DONE, ERROR, INFO, SUCCESS, WARN,
};

/// 編譯期嵌入的內建資源（由 build.rs 產生）。
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_resources.rs"));
}

pub const GAME_NAME: &str = "大富翁3";
pub const TOTAL_STEPS: u32 = 6;

const RESOURCE_FOLDERS: [&str; 3] = ["EVENTVOC", "NEWSVOC", "SCREEN"];

// =====================================================================
// 步驟 1：釋放內建資源
// =====================================================================

/// 把內建的 EVENTVOC / NEWSVOC / SCREEN 釋放到遊戲目錄。
///
/// 回傳「本次由程式建立、事後應清除」的資料夾清單。**原本就存在的資料夾不會被列入**——
/// 那是使用者自己的東西，我們只覆寫內容，不負責刪掉它。
pub fn extract_bundled_folders(
    target_dir: &Path,
    reporter: &dyn Reporter,
    step: u32,
) -> io::Result<Vec<PathBuf>> {
    reporter.log("開始檢查並釋放內建資源檔...", INFO, Some(step));

    let mut created = Vec::new();

    for folder in RESOURCE_FOLDERS {
        let dest = target_dir.join(folder);
        let existed = dest.exists();
        fs::create_dir_all(&dest)?;

        for (owner, name, bytes) in embedded::EMBEDDED {
            if *owner == folder {
                fs::write(dest.join(name), bytes)?;
            }
        }

        if existed {
            reporter.log(&format!("覆寫/更新現有資源: {folder}"), INFO, Some(step));
        } else {
            created.push(dest);
            reporter.log(&format!("已釋放資源: {folder}"), INFO, Some(step));
        }
    }

    Ok(created)
}

/// 清除步驟 1 建立的暫存資料夾。
pub fn cleanup_folders(created: &[PathBuf], reporter: &dyn Reporter) {
    if created.is_empty() {
        return;
    }
    reporter.info("開始執行毀屍滅跡 (清理暫存檔案)...");
    for dir in created {
        match fs::remove_dir_all(dir) {
            Ok(()) => reporter.info(&format!("已刪除暫存資料夾: {}", dir.display())),
            Err(err) => reporter.log(
                &format!("清理 {} 失敗: {err}", dir.display()),
                WARN,
                None,
            ),
        }
    }
}

// =====================================================================
// 步驟 3：主程式
// =====================================================================

/// `RICH3.EXE` 的 14 條特徵碼。
///
/// 磁片版、重訂光碟版、Steam 典藏版的偏移位址不同，全部列出逐一嘗試——任一版本本來
/// 就只會命中屬於它的那幾條，其餘顯示「跳過」是正常的。Steam 典藏版實測為 11/14。
fn exe_rules(total_days: usize) -> Vec<Rule> {
    // 日曆天數是執行期才決定的，中間兩個位元組必須放行
    let days = (total_days as u16).to_le_bytes();
    let mut cald_replacement = vec![0xB9];
    cald_replacement.extend_from_slice(&days);
    cald_replacement.extend_from_slice(&[0xC4, 0x7E, 0x0A]);

    vec![
        Rule::new(
            "多人競賽也可一個人玩",
            &[("3B 46 C8 7F 0E", "3B 46 C8 90 90")],
        ),
        Rule::new(
            "修正日期二月跳三月問題",
            &[("75 05 C7 46 EC 1D 00 8B", "75 14 C7 46 EC 1D 00 8B")],
        ),
        Rule::new(
            "修正日曆超過 32KB 無效",
            &[("48 D1 E0 D1 E0 99", "48 99 D1 E0 D1 E0")],
        ),
        Rule::wildcard(
            &format!("自動變更 CALD.A 搜尋組數 ({total_days} 天)"),
            "B9 .. .. C4 7E 0A",
            cald_replacement,
        ),
        Rule::new(
            "命運事件「賣天婦羅」獎金",
            &[("81 C1 C8 00 83 D3 00", "81 C1 D0 07 83 D3 00")],
        ),
        Rule::new(
            "新聞事件「表彰先進」獎金 (上)",
            &[
                ("81 C1 B8 0B 83 D3 00 89 86 2C FE", "81 C1 88 13 83 D3 00 89 86 2C FE"),
                ("81 C1 B8 0B 83 D3 00 89 86 2A FE", "81 C1 88 13 83 D3 00 89 86 2A FE"),
                ("81 C1 B8 0B 83 D3 00 89 86 34 FE", "81 C1 88 13 83 D3 00 89 86 34 FE"),
            ],
        ),
        Rule::new(
            "新聞事件「表彰先進」獎金 (下)",
            &[
                ("C7 86 2E FE DD 01 8D 86 32 FE", "C7 86 2E FE DE 01 8D 86 32 FE"),
                ("C7 86 2C FE DD 01 8D 86 30 FE", "C7 86 2C FE DE 01 8D 86 30 FE"),
                ("C7 86 36 FE DD 01 8D 86 3A FE", "C7 86 36 FE DE 01 8D 86 3A FE"),
            ],
        ),
        Rule::new(
            "修正住院/坐牢免付過路費位置",
            &[("68 2A 02 68 2A 02", "68 2A 02 68 2C 02")],
        ),
        Rule::new(
            "破解顏色密碼 (磁片版)",
            &[("83 3E BC 00 02 74 03", "83 3E BC 00 02 EB 03")],
        ),
        Rule::new(
            "破解光碟檢查 (相容項 1)",
            &[("83 7E EA 06 74 10", "83 7E EA 06 EB 10")],
        ),
        Rule::new("破解光碟檢查 (相容項 2)", &[("0A FF 75 08", "0A FF 90 90")]),
        Rule::new(
            "破解光碟檢查 (相容項 3)",
            &[("E8 BB 03 EB 2F", "B0 ED 90 EB 2F")],
        ),
        Rule::new(
            "破解光碟檢查 (相容項 4)",
            &[("56 11 02 00 3A 5C", "56 11 01 00 5C 5C")],
        ),
        Rule::new(
            "破解光碟檢查 (相容項 5)",
            &[("C4 7E 06 98 AB", "C4 7E 06 90 AB")],
        ),
    ]
}

pub fn patch_exe(
    target_dir: &Path,
    total_days: usize,
    reporter: &dyn Reporter,
    step: u32,
) -> io::Result<bool> {
    reporter.log("開始尋找主程式並進行修改...", INFO, Some(step));

    // 大富翁3 的主程式為 RICH3.EXE / RICH3S.EXE
    let exe_target = match find_target(
        target_dir,
        &["RICH3.EXE", "RICH3S.EXE", "rich3.exe", "rich3s.exe"],
    ) {
        Some(path) => path,
        None => {
            reporter.log(
                "找不到 RICH3.EXE 或 RICH3S.EXE！請確認檔案在目標目錄。",
                ERROR,
                None,
            );
            return Ok(false);
        }
    };

    reporter.info(&format!("找到主程式：{}", exe_target.display()));
    backup_file(&exe_target, reporter)?;

    patch_binary(
        &exe_target,
        &exe_rules(total_days),
        ReplaceMode::First,
        reporter,
    )
}

// =====================================================================
// 步驟 4：地圖物價與地點資料
// =====================================================================

/// `MAP.MKF` 的修正規則：兩條物價、五條地點資料誤植。
///
/// 每張地圖的 `#02 座標地點轉換表`為 50×50 格、每格 10 byte，五個小端 u16：
/// 路段編號／地點形態／作用地點 X／作用地點 Y／資訊編號。格子的索引為 `x + y * 50`。
/// 名稱索引 = 路段編號 + 0x54，帶了路段編號遊戲就會顯示地名。
fn map_rules() -> Vec<Rule> {
    vec![
        Rule::new(
            "台北新生南路蓋屋價 3600 -> 360",
            &[("FC 08 00 00 10 0E", "FC 08 00 00 68 01")],
        ),
        Rule::new(
            "台北建國北路二層房過路費 200 -> 2000",
            &[("84 03 00 00 C8 00", "84 03 00 00 D0 07")],
        ),
        // ── 原版地點資料的誤植 ─────────────────────────────────────
        // 以下每條特徵碼都在整份 MAP.MKF（1,944,084 byte）裡驗證過只出現一次。
        // 修的都是「多餘」的資料，正常運作的格子一格都不該受影響。
        Rule::new(
            "大陸 (1,0) 清除誤植的路段39與形態33（顯示成中山南路）",
            &[(
                "27 00 21 00 02 00 04 00 00 00",
                "00 00 00 00 00 00 00 00 00 00",
            )],
        ),
        // 台北那三格是建地 81/82/83 的建築物圖，只清前 2 byte 的路段編號；
        // 資訊編號 81/82/83 是它們與地產的關聯，動了會影響買賣與收租。
        Rule::new(
            "台北 (29,30) 清除誤植的路段224（顯示成新生北路）",
            &[(
                "E0 00 00 00 00 00 00 00 51 00",
                "00 00 00 00 00 00 00 00 51 00",
            )],
        ),
        Rule::new(
            "台北 (29,31) 清除誤植的路段224（顯示成新生北路）",
            &[(
                "E0 00 00 00 00 00 00 00 52 00",
                "00 00 00 00 00 00 00 00 52 00",
            )],
        ),
        Rule::new(
            "台北 (29,32) 清除誤植的路段224（顯示成新生北路）",
            &[(
                "E0 00 00 00 00 00 00 00 53 00",
                "00 00 00 00 00 00 00 00 53 00",
            )],
        ),
        // ⚠ 大陸公園多出來的上排那兩格相鄰，必須寫成**一條**規則：
        //   拆成兩條的話，第一條改完之後第二條的特徵碼就對不上了。
        //   前後各帶一格作為上下文，靠它們湊出唯一的特徵碼。
        Rule::new(
            "大陸 (46,36)(47,36) 清除誤填成公園的欄位",
            &[(
                "00 00 00 00 00 00 00 00 00 00 \
                 0C 00 01 00 00 00 00 00 F9 00 \
                 0C 00 01 00 00 00 00 00 F9 00 \
                 00 00 00 00 00 00 00 00 D2 00",
                "00 00 00 00 00 00 00 00 00 00 \
                 00 00 00 00 00 00 00 00 00 00 \
                 00 00 00 00 00 00 00 00 00 00 \
                 00 00 00 00 00 00 00 00 D2 00",
            )],
        ),
    ]
}

pub fn patch_map_mkf(target_dir: &Path, reporter: &dyn Reporter, step: u32) -> io::Result<bool> {
    reporter.log("開始處理 MAP.MKF 修正物價...", INFO, Some(step));

    let map_target = match find_target(target_dir, &["MAP.MKF", "map.mkf"]) {
        Some(path) => path,
        None => {
            reporter.log("找不到 MAP.MKF！跳過地圖檔修改。", WARN, None);
            return Ok(false);
        }
    };

    backup_file(&map_target, reporter)?;

    // 資料檔與 EXE 不同：同一筆數值可能合法地出現多次且都該改
    patch_binary(&map_target, &map_rules(), ReplaceMode::All, reporter)
}

// =====================================================================
// 步驟 5、6：MKF 資源注入
// =====================================================================

/// 不分大小寫尋找目錄中的檔案或子目錄。
fn find_entry(target_dir: &Path, name: &str, want_dir: bool) -> Option<PathBuf> {
    let entries = fs::read_dir(target_dir).ok()?;
    for entry in entries.flatten() {
        let matches_kind = entry.path().is_dir() == want_dir;
        let same_name = entry
            .file_name()
            .to_str()
            .map(|n| n.eq_ignore_ascii_case(name))
            .unwrap_or(false);
        if matches_kind && same_name {
            return Some(entry.path());
        }
    }
    None
}

/// 從檔名尾端取出序號，例如 `screen_19.bin` → 19、`NEWSVOC_001.voc` → 1。
fn index_of(file_name: &str, prefix: &str) -> Option<usize> {
    let lower = file_name.to_ascii_lowercase();
    let prefix = format!("{}_", prefix.to_ascii_lowercase());
    let rest = lower.strip_prefix(&prefix)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// 步驟 5：畫面封裝檔。
///
/// ⚠ 索引是 **1 起算**：`screen_19.bin` 對應第 19 塊，也就是陣列的 `[18]`。
/// 這與語音 MKF 的 0 起算不同，是 Python 版就有的差異，移植時原樣保留。
pub fn patch_screen_mkf(target_dir: &Path, reporter: &dyn Reporter, step: u32) -> io::Result<bool> {
    reporter.log("開始處理畫面封裝檔 SCREEN.MKF...", INFO, Some(step));

    let mkf_path = match find_entry(target_dir, "screen.mkf", false) {
        Some(path) => path,
        None => {
            reporter.log("目標目錄找不到 SCREEN.MKF 啦！", ERROR, None);
            return Ok(false);
        }
    };

    let patch_dir = match find_entry(target_dir, "screen", true) {
        Some(path) => path,
        None => {
            fs::create_dir_all(target_dir.join("screen"))?;
            reporter.log("沒找到 screen 資料夾，幫你建一個。有檔案再來跑！", WARN, None);
            return Ok(false);
        }
    };

    let mut chunks = mkf::read_chunks(&mkf_path)?;

    let mut files: Vec<(usize, PathBuf)> = fs::read_dir(&patch_dir)?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_ascii_lowercase();
            if !name.ends_with(".bin") {
                return None;
            }
            Some((index_of(&name, "screen")?, path))
        })
        .collect();
    files.sort();

    let mut patched = 0usize;
    for (number, path) in files {
        let target_idx = number.checked_sub(1);
        match target_idx {
            Some(idx) if idx < chunks.len() => {
                reporter.info(&format!(
                    "注入畫面: {} -> 索引 {number}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
                chunks[idx] = fs::read(&path)?;
                patched += 1;
            }
            _ => {}
        }
    }

    if patched == 0 {
        reporter.log("screen 資料夾沒發現可用的 .bin 檔，白忙一場。", WARN, None);
        return Ok(false);
    }

    backup_file(&mkf_path, reporter)?;
    mkf::write_chunks(&mkf_path, &chunks)?;

    reporter.log(
        &format!("畫面重組完成！共貫穿了 {patched} 張。"),
        SUCCESS,
        None,
    );
    Ok(true)
}

/// 步驟 6：語音封裝檔。索引 **0 起算**。
pub fn patch_audio_mkf(
    target_dir: &Path,
    target_name: &str,
    reporter: &dyn Reporter,
    step: u32,
) -> io::Result<bool> {
    reporter.log(
        &format!("開始處理語音封裝檔 {target_name}.MKF..."),
        INFO,
        Some(step),
    );

    let mkf_path = match find_entry(target_dir, &format!("{target_name}.mkf"), false) {
        Some(path) => path,
        None => {
            reporter.log(&format!("找不到 {target_name}.MKF！"), ERROR, None);
            return Ok(false);
        }
    };

    let patch_dir = match find_entry(target_dir, target_name, true) {
        Some(path) => path,
        None => {
            reporter.log(&format!("沒找到 {target_name} 資料夾，跳過。"), WARN, None);
            return Ok(false);
        }
    };

    let mut chunks = mkf::read_chunks(&mkf_path)?;

    let mut files: Vec<(usize, PathBuf, String)> = fs::read_dir(&patch_dir)?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            if !name.to_ascii_lowercase().ends_with(".voc") {
                return None;
            }
            Some((index_of(&name, target_name)?, path, name))
        })
        .collect();
    files.sort();

    let mut patched = 0usize;
    for (idx, path, name) in files {
        if idx < chunks.len() {
            reporter.info(&format!("注入音檔: {name} -> 索引 {idx}"));
            chunks[idx] = fs::read(&path)?;
            patched += 1;
        } else {
            reporter.log(&format!("序數 {idx} 超過原始總數，跳過。"), WARN, None);
        }
    }

    if patched == 0 {
        reporter.log(
            &format!("{target_name} 資料夾無可用檔案，白忙一場。"),
            WARN,
            None,
        );
        return Ok(false);
    }

    backup_file(&mkf_path, reporter)?;
    mkf::write_chunks(&mkf_path, &chunks)?;

    reporter.log(
        &format!(
            "{} 重組完成。替換了 {patched} 個音檔。",
            mkf_path.display()
        ),
        SUCCESS,
        None,
    );
    Ok(true)
}

// =====================================================================
// 主幹流程
// =====================================================================

/// 執行全套 patch，回傳給使用者看的執行摘要。
///
/// `today` 由呼叫端傳入，讓 oracle 比對能固定基準日期（見 calendar 模組）。
pub fn run_patch(
    target_dir: &Path,
    today: NaiveDate,
    reporter: &dyn Reporter,
) -> io::Result<String> {
    // 無論成功失敗都要清掉自己建立的暫存資料夾，所以先接住結果再處理
    let created = extract_bundled_folders(target_dir, reporter, 1)?;

    let outcome = (|| -> io::Result<[(&str, bool); 5]> {
        let total_days = calendar::generate(target_dir, today, reporter, 2)?;
        let exe_res = patch_exe(target_dir, total_days, reporter, 3)?;
        let map_res = patch_map_mkf(target_dir, reporter, 4)?;
        let screen_res = patch_screen_mkf(target_dir, reporter, 5)?;
        let voc_news = patch_audio_mkf(target_dir, "NEWSVOC", reporter, 6)?;
        let voc_event = patch_audio_mkf(target_dir, "EVENTVOC", reporter, 6)?;
        Ok([
            ("主程式 (EXE)", exe_res),
            ("地圖檔 (MAP)", map_res),
            ("畫面檔 (SCREEN)", screen_res),
            ("新聞語音 (NEWSVOC)", voc_news),
            ("事件語音 (EVENTVOC)", voc_event),
        ])
    })();

    cleanup_folders(&created, reporter);

    let results = outcome?;
    let report = format_report(&results);
    reporter.log("所有任務完工！爽啦！", DONE, Some(TOTAL_STEPS));

    Ok(format!(
        "{GAME_NAME} 全套 Patch 執行完畢！\n\n【執行摘要】\n{report}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::Match;

    #[test]
    fn 特徵碼的替換長度必須與原始長度相同() {
        for rule in exe_rules(14612).into_iter().chain(map_rules()) {
            for candidate in &rule.matches {
                let (from_len, to_len) = match candidate {
                    Match::Exact { from, to } => (from.len(), to.len()),
                    Match::Wildcard { pattern, to } => (pattern.len(), to.len()),
                };
                assert_eq!(from_len, to_len, "特徵碼「{}」長度不一致", rule.name);
            }
        }
    }

    #[test]
    fn 共十四條特徵碼() {
        assert_eq!(exe_rules(14612).len(), 14);
    }

    #[test]
    fn 地圖檔共七條特徵碼() {
        assert_eq!(map_rules().len(), 7);
    }

    /// MAP.MKF 的修正一共只該動 13 個 byte：大陸 (1,0) 四個、台北三格各一個、
    /// 大陸公園那兩格各三個。數字對不上就是有規則的替換內容寫錯了。
    #[test]
    fn 地點誤植修正一共只動十三個位元組() {
        let changed: usize = map_rules()
            .iter()
            .filter(|rule| rule.name.contains("清除誤"))
            .flat_map(|rule| &rule.matches)
            .map(|candidate| {
                let Match::Exact { from, to } = candidate else {
                    panic!("地點誤植修正不該用萬用比對");
                };
                from.iter().zip(to).filter(|(a, b)| a != b).count()
            })
            .sum();
        assert_eq!(changed, 13);
    }

    #[test]
    fn 日曆天數會寫進搜尋組數的特徵碼() {
        let rules = exe_rules(14612);
        let cald = rules
            .iter()
            .find(|r| r.name.contains("CALD.A"))
            .expect("找不到 CALD.A 那條");
        let Match::Wildcard { to, .. } = &cald.matches[0] else {
            panic!("CALD.A 那條應該是萬用比對");
        };
        // 14612 = 0x3914，小端為 14 39
        assert_eq!(to, &vec![0xB9, 0x14, 0x39, 0xC4, 0x7E, 0x0A]);
        assert!(cald.name.contains("14612 天"));
    }

    #[test]
    fn 序號解析() {
        assert_eq!(index_of("screen_19.bin", "screen"), Some(19));
        assert_eq!(index_of("NEWSVOC_001.voc", "NEWSVOC"), Some(1));
        assert_eq!(index_of("EVENTVOC_116.voc", "eventvoc"), Some(116));
        assert_eq!(index_of("readme.txt", "screen"), None);
        assert_eq!(index_of("screen_abc.bin", "screen"), None);
    }

    #[test]
    fn 內建資源都在() {
        let mut counts = std::collections::HashMap::new();
        for (folder, _, bytes) in embedded::EMBEDDED {
            *counts.entry(*folder).or_insert(0usize) += 1;
            assert!(!bytes.is_empty(), "嵌入的資源不該是空的");
        }
        assert_eq!(counts.get("EVENTVOC"), Some(&11));
        assert_eq!(counts.get("NEWSVOC"), Some(&8));
        assert_eq!(counts.get("SCREEN"), Some(&1));
    }
}
