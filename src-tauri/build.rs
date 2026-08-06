use std::fmt::Write as _;
use std::path::Path;

/// 把 EVENTVOC / NEWSVOC / SCREEN 三個資源資料夾在編譯期嵌進執行檔。
///
/// **為什麼不用 Tauri 的 bundle resources**：那個機制是把檔案放在安裝目錄旁邊，
/// 只對安裝版有效。我們的主力產物是 portable 單檔 exe，外部檔案拿不到。
/// Python 版靠 PyInstaller onefile 把資源打進去、執行時解到暫存目錄，
/// 這裡用 `include_bytes!` 達到同樣效果——約 440 KB，換來單檔可攜。
fn embed_resources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("找不到專案根目錄");

    let mut generated = String::from(
        "// 由 build.rs 自動產生，請勿手動編輯。\n\
         /// (資料夾, 檔名, 內容)\n\
         pub static EMBEDDED: &[(&str, &str, &[u8])] = &[\n",
    );

    for folder in ["EVENTVOC", "NEWSVOC", "SCREEN"] {
        let dir = root.join(folder);
        println!("cargo:rerun-if-changed={}", dir.display());

        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("讀不到資源資料夾 {}：{err}", dir.display()))
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .map(|e| e.path())
            .collect();
        entries.sort();

        for path in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("資源檔名不是合法的 UTF-8");
            writeln!(
                generated,
                "    (\"{folder}\", \"{name}\", include_bytes!(r\"{}\")),",
                path.display()
            )
            .expect("產生程式碼失敗");
        }
    }
    generated.push_str("];\n");

    let out = Path::new(&std::env::var("OUT_DIR").expect("沒有 OUT_DIR"))
        .join("embedded_resources.rs");
    std::fs::write(&out, generated).expect("寫入產生的程式碼失敗");
}

fn main() {
    embed_resources();
    tauri_build::build()
}
