// 進入點。實作全部在 lib.rs，這樣 patch 引擎才能被測試與 oracle 比對引用。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rich3_patch::run()
}
