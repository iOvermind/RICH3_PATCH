// Rich Patch Series 的共用 patch 引擎。
//
// 這個模組刻意**不依賴 Tauri**：所有輸出都走 `Reporter` trait。這樣同一份邏輯既能
// 在應用程式裡跑，也能在測試裡跑，更重要的是能拿來和 Python 版做逐位元組比對
// （Python 版是驗證這份實作正確性的基準，見 DEVELOPER.md §8）。
//
// 行為刻意與 Python 版 `main.py` 逐項對齊，包含日誌字串——這樣兩邊的輸出可以直接對照。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 日誌等級。字串與 Python 版一致。
pub const INFO: &str = "INFO";
pub const WARN: &str = "WARN";
pub const ERROR: &str = "ERROR";
pub const SUCCESS: &str = "SUCCESS";
pub const FATAL: &str = "FATAL";
pub const DONE: &str = "DONE";

/// 日誌與進度的輸出管道。
///
/// `step` 有值時代表這是一個步驟的開始，UI 應同時推進進度條；`None` 則是細節訊息。
pub trait Reporter {
    fn log(&self, message: &str, level: &str, step: Option<u32>);

    fn info(&self, message: &str) {
        self.log(message, INFO, None);
    }
}

/// 一種比對方式。
pub enum Match {
    /// 逐位元組完全相同。
    Exact { from: Vec<u8>, to: Vec<u8> },
    /// 允許萬用位元組（`..`）的比對。
    ///
    /// 對應 Python 版用 regex 寫的那種特徵碼——例如日曆天數是執行期才決定的，
    /// 特徵碼中間兩個位元組必須放行。
    Wildcard { pattern: Vec<Option<u8>>, to: Vec<u8> },
}

impl Match {
    fn len(&self) -> usize {
        match self {
            Match::Exact { from, .. } => from.len(),
            Match::Wildcard { pattern, .. } => pattern.len(),
        }
    }

    fn replacement(&self) -> &[u8] {
        match self {
            Match::Exact { to, .. } | Match::Wildcard { to, .. } => to,
        }
    }

    /// 在 `data` 中尋找第一個符合的位置。
    fn find(&self, data: &[u8]) -> Option<usize> {
        match self {
            Match::Exact { from, .. } => find_sub(data, from),
            Match::Wildcard { pattern, .. } => {
                if pattern.is_empty() || pattern.len() > data.len() {
                    return None;
                }
                data.windows(pattern.len()).position(|window| {
                    window
                        .iter()
                        .zip(pattern)
                        .all(|(actual, expected)| expected.map_or(true, |byte| byte == *actual))
                })
            }
        }
    }
}

/// 一條特徵碼修正。
///
/// `matches` 是多組比對方式，依序嘗試，命中第一組就停——同一個修正在不同遊戲版本
/// 有不同的偏移，所以會有多組。
pub struct Rule {
    pub name: String,
    pub matches: Vec<Match>,
}

impl Rule {
    /// 逐位元組比對。`pairs` 為多組「原始 → 替換」的十六進位字串。
    pub fn new(name: &str, pairs: &[(&str, &str)]) -> Self {
        Rule {
            name: name.to_string(),
            matches: pairs
                .iter()
                .map(|(from, to)| Match::Exact {
                    from: hex(from),
                    to: hex(to),
                })
                .collect(),
        }
    }

    /// 帶萬用位元組的比對。`pattern` 中的 `..` 代表任意位元組。
    pub fn wildcard(name: &str, pattern: &str, to: Vec<u8>) -> Self {
        let pattern = hex_pattern(pattern);
        assert_eq!(
            pattern.len(),
            to.len(),
            "特徵碼「{name}」的比對長度與替換長度不一致"
        );
        Rule {
            name: name.to_string(),
            matches: vec![Match::Wildcard { pattern, to }],
        }
    }
}

/// 替換範圍。
///
/// Python 版依副檔名決定：EXE 只換第一處（特徵碼是特定指令位置，全域替換可能誤傷），
/// MKF 換全部（資料檔裡同一筆數值可能合法地出現多次且都該改）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReplaceMode {
    First,
    All,
}

/// 把 "83 3E E8 10 00 7E" 這種寫法轉成位元組。
///
/// 特徵碼是手抄的，維持與 Python 版 `bytes.fromhex()` 相同的可讀寫法，方便逐條核對。
pub fn hex(s: &str) -> Vec<u8> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        cleaned.len() % 2 == 0,
        "特徵碼長度必須是偶數：{s}"
    );
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .unwrap_or_else(|_| panic!("特徵碼不是合法的十六進位：{s}"))
        })
        .collect()
}

/// 把 "B9 .. .. C4 7E 0A" 轉成比對樣式，`..` 代表任意位元組。
///
/// 寫法刻意貼近 Python 版的 regex（`b"\xB9..\xC4\x7E\x0A"`），逐條核對時比較不會看漏。
pub fn hex_pattern(s: &str) -> Vec<Option<u8>> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(cleaned.len() % 2 == 0, "特徵碼長度必須是偶數：{s}");
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            let pair = &cleaned[i..i + 2];
            if pair == ".." {
                None
            } else {
                Some(
                    u8::from_str_radix(pair, 16)
                        .unwrap_or_else(|_| panic!("特徵碼不是合法的十六進位：{s}")),
                )
            }
        })
        .collect()
}

/// 在 `haystack` 中尋找 `needle` 的起始位置。
fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 取代位元組序列。與 Python 的 `bytes.replace()` 一樣，由左至右、不重疊。
fn replace_bytes(data: &[u8], from: &[u8], to: &[u8], mode: ReplaceMode) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut cursor = 0;

    while cursor < data.len() {
        match find_sub(&data[cursor..], from) {
            Some(pos) => {
                out.extend_from_slice(&data[cursor..cursor + pos]);
                out.extend_from_slice(to);
                cursor += pos + from.len();
                if mode == ReplaceMode::First {
                    break;
                }
            }
            None => break,
        }
    }
    out.extend_from_slice(&data[cursor..]);
    out
}

/// 在目標目錄中尋找第一個存在的檔案。
///
/// Windows 的檔案系統本身不分大小寫，這裡列出多種寫法是為了在其他平台上也能運作。
pub fn find_target(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

/// 備份成 `<原檔名>.bak`。
///
/// **若 `.bak` 已存在則不覆蓋**——使用者重複執行是常態，覆蓋備份會讓他再也回不到原版。
/// 這條規則不可更動（見 DEVELOPER.md §9.4）。
pub fn backup_file(path: &Path, reporter: &dyn Reporter) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let mut bak = path.as_os_str().to_os_string();
    bak.push(".bak");
    let bak = PathBuf::from(bak);

    if bak.exists() {
        reporter.info(&format!(
            "{} 備份已存在，跳過覆蓋以保留最原始檔案",
            bak.display()
        ));
    } else {
        fs::copy(path, &bak)?;
        reporter.info(&format!("已建立備份 {}", bak.display()));
    }
    Ok(true)
}

/// 對單一檔案套用一組特徵碼修正。
///
/// 回傳是否真的寫入了變更。
pub fn patch_binary(
    path: &Path,
    rules: &[Rule],
    mode: ReplaceMode,
    reporter: &dyn Reporter,
) -> io::Result<bool> {
    let original = fs::read(path)?;
    reporter.info(&format!("開始分析與 Patch {} ...", path.display()));

    let mut modified = original.clone();
    let mut success_count = 0usize;

    for rule in rules {
        let mut hit = false;
        for candidate in &rule.matches {
            match candidate {
                Match::Exact { from, to } => {
                    if find_sub(&modified, from).is_some() {
                        modified = replace_bytes(&modified, from, to, mode);
                        hit = true;
                    }
                }
                Match::Wildcard { .. } => {
                    if let Some(pos) = candidate.find(&modified) {
                        // 萬用比對一律只換第一處，與 Python 版 re.sub(..., count=1) 一致
                        let end = pos + candidate.len();
                        modified.splice(pos..end, candidate.replacement().iter().copied());
                        hit = true;
                    }
                }
            }
            if hit {
                break;
            }
        }

        if hit {
            reporter.info(&format!("[成功] {}", rule.name));
            success_count += 1;
        } else {
            reporter.log(
                &format!("[跳過] {} (找不到特徵碼或已修改)", rule.name),
                WARN,
                None,
            );
        }
    }

    if original != modified {
        fs::write(path, &modified)?;
        reporter.log(
            &format!(
                "[完成] {} 已儲存修改 ({}/{} 項).",
                path.display(),
                success_count,
                rules.len()
            ),
            SUCCESS,
            None,
        );
        Ok(true)
    } else {
        reporter.log(
            &format!("[提示] {} 沒有發生任何變更。", path.display()),
            WARN,
            None,
        );
        Ok(false)
    }
}

/// 把各步驟成果整理成統一格式的摘要。
pub fn format_report(results: &[(&str, bool)]) -> String {
    results
        .iter()
        .map(|(label, ok)| {
            let mark = if *ok { "✅" } else { "⚠️" };
            let state = if *ok { "成功處理" } else { "未變動或失敗" };
            format!("{mark} {label}: {state}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// 把日誌收進 Vec，供測試斷言。
    struct Collector {
        lines: RefCell<Vec<String>>,
    }

    impl Collector {
        fn new() -> Self {
            Collector {
                lines: RefCell::new(Vec::new()),
            }
        }
        fn lines(&self) -> Vec<String> {
            self.lines.borrow().clone()
        }
    }

    impl Reporter for Collector {
        fn log(&self, message: &str, level: &str, _step: Option<u32>) {
            self.lines.borrow_mut().push(format!("[{level}] {message}"));
        }
    }

    #[test]
    fn hex_忽略空白() {
        assert_eq!(hex("83 3E E8 10"), vec![0x83, 0x3E, 0xE8, 0x10]);
        assert_eq!(hex("833EE810"), vec![0x83, 0x3E, 0xE8, 0x10]);
    }

    #[test]
    fn replace_first_只換第一處() {
        let data = vec![1, 2, 3, 1, 2, 3];
        let out = replace_bytes(&data, &[1, 2], &[9, 9], ReplaceMode::First);
        assert_eq!(out, vec![9, 9, 3, 1, 2, 3]);
    }

    #[test]
    fn replace_all_換全部() {
        let data = vec![1, 2, 3, 1, 2, 3];
        let out = replace_bytes(&data, &[1, 2], &[9, 9], ReplaceMode::All);
        assert_eq!(out, vec![9, 9, 3, 9, 9, 3]);
    }

    #[test]
    fn replace_不重疊() {
        // Python 的 bytes.replace 也是不重疊地由左至右掃描
        let data = vec![1, 1, 1, 1];
        let out = replace_bytes(&data, &[1, 1], &[2, 2], ReplaceMode::All);
        assert_eq!(out, vec![2, 2, 2, 2]);
    }

    #[test]
    fn replace_長度不同也正確() {
        let data = vec![0, 1, 2, 3, 0];
        let out = replace_bytes(&data, &[1, 2, 3], &[7], ReplaceMode::First);
        assert_eq!(out, vec![0, 7, 0]);
    }

    #[test]
    fn 備份不覆蓋既有的_bak() {
        let dir = std::env::temp_dir().join(format!("rich2_patch_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let target = dir.join("RUN.EXE");
        fs::write(&target, b"original").unwrap();

        let reporter = Collector::new();
        backup_file(&target, &reporter).unwrap();
        assert_eq!(fs::read(dir.join("RUN.EXE.bak")).unwrap(), b"original");

        // 檔案被改過之後再備份一次，.bak 必須維持最原始的內容
        fs::write(&target, b"patched").unwrap();
        backup_file(&target, &reporter).unwrap();
        assert_eq!(fs::read(dir.join("RUN.EXE.bak")).unwrap(), b"original");

        let lines = reporter.lines();
        assert!(lines[0].contains("已建立備份"));
        assert!(lines[1].contains("備份已存在"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn 沒有命中就不寫檔() {
        let dir = std::env::temp_dir().join(format!("rich2_patch_test_nohit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let target = dir.join("RUN.EXE");
        fs::write(&target, b"\x00\x01\x02\x03").unwrap();

        let rules = vec![Rule::new("不存在的特徵碼", &[("AA BB", "CC DD")])];
        let reporter = Collector::new();
        let changed = patch_binary(&target, &rules, ReplaceMode::First, &reporter).unwrap();

        assert!(!changed);
        assert_eq!(fs::read(&target).unwrap(), b"\x00\x01\x02\x03");
        assert!(reporter.lines().iter().any(|l| l.contains("[跳過]")));
        assert!(reporter.lines().iter().any(|l| l.contains("沒有發生任何變更")));

        fs::remove_dir_all(&dir).unwrap();
    }
}
