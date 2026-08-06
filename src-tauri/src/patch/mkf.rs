// MKF 封裝檔的拆解與重組。
//
// MKF 的結構是：開頭一張 32 位小端的偏移量索引表，之後接各區塊的原始資料。
// 索引表有 n+1 筆——最後一筆是檔案總長度，所以第 i 塊的範圍是 [offset[i], offset[i+1])。
// 索引表本身的長度由第一筆偏移量決定（第一塊資料的起點就是索引表的結尾）。
//
// 注入的資料長度與原本不同，因此**不能就地覆寫**，必須整份重算偏移量再寫回。

use std::fs;
use std::io;
use std::path::Path;

/// 讀取 MKF 並依索引表切成一塊塊資料。
pub fn read_chunks(path: &Path) -> io::Result<Vec<Vec<u8>>> {
    let data = fs::read(path)?;

    let mut offsets: Vec<usize> = Vec::new();
    let mut cursor = 0usize;

    while cursor + 4 <= data.len() {
        let offset = u32::from_le_bytes([
            data[cursor],
            data[cursor + 1],
            data[cursor + 2],
            data[cursor + 3],
        ]) as usize;
        offsets.push(offset);
        cursor += 4;

        // 讀到第一筆偏移量指向的位置就代表索引表結束了
        if offsets.len() > 1 && cursor >= offsets[0] {
            break;
        }
    }

    if offsets.len() < 2 {
        return Ok(Vec::new());
    }

    let mut chunks = Vec::with_capacity(offsets.len() - 1);
    for pair in offsets.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        // 損壞或超出範圍的索引一律當成空區塊，與 Python 的切片語意一致
        if start <= end && end <= data.len() {
            chunks.push(data[start..end].to_vec());
        } else {
            chunks.push(Vec::new());
        }
    }
    Ok(chunks)
}

/// 重建索引表並把資料塊重新封裝寫回。
pub fn write_chunks(path: &Path, chunks: &[Vec<u8>]) -> io::Result<()> {
    let mut out = Vec::new();

    // 索引表佔 (n+1) * 4 個位元組，第一塊資料緊接在後
    let mut offset = (chunks.len() + 1) * 4;
    let mut offsets = Vec::with_capacity(chunks.len() + 1);
    for chunk in chunks {
        offsets.push(offset);
        offset += chunk.len();
    }
    offsets.push(offset); // 最後一筆是總長度

    for value in &offsets {
        out.extend_from_slice(&(*value as u32).to_le_bytes());
    }
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }

    fs::write(path, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rich3_mkf_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn 拆解後再重組應完全還原() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("TEST.MKF");

        let original = vec![
            vec![1u8, 2, 3],
            vec![4, 5],
            vec![6, 7, 8, 9],
        ];
        write_chunks(&path, &original).unwrap();

        let read_back = read_chunks(&path).unwrap();
        assert_eq!(read_back, original);

        // 再寫一次，位元組應與第一次完全相同
        let first = fs::read(&path).unwrap();
        write_chunks(&path, &read_back).unwrap();
        assert_eq!(fs::read(&path).unwrap(), first);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn 索引表會隨著區塊長度改變而重算() {
        let dir = temp_dir("reindex");
        let path = dir.join("TEST.MKF");

        write_chunks(&path, &[vec![1, 2, 3], vec![4, 5]]).unwrap();
        let mut chunks = read_chunks(&path).unwrap();

        // 把第一塊換成更長的資料——偏移量必須整體後移
        chunks[0] = vec![9; 100];
        write_chunks(&path, &chunks).unwrap();

        let after = read_chunks(&path).unwrap();
        assert_eq!(after[0].len(), 100);
        assert_eq!(after[1], vec![4, 5], "後面的區塊不可被破壞");

        let raw = fs::read(&path).unwrap();
        let first_offset = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
        assert_eq!(first_offset, 12, "兩塊資料的索引表應為 (2+1)*4 = 12 位元組");
        assert_eq!(raw.len(), 12 + 100 + 2);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn 空檔案不會造成崩潰() {
        let dir = temp_dir("empty");
        let path = dir.join("EMPTY.MKF");
        fs::write(&path, b"").unwrap();
        assert!(read_chunks(&path).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
