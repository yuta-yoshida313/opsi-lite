// Markdown(GFM)テーブルの解析とセル編集。
//
// プレビュー上でテーブルをグリッド表示し、セル単位で編集できるようにするための
// 解析(detect)と、本文への書き戻し(set_cell / get_cell)を提供する。

/// 列の揃え。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

/// 検出された 1 つのテーブル。
#[derive(Debug, Clone)]
pub struct Table {
    /// ヘッダ行の本文行インデックス(0始まり)。区切り行は header_line + 1。
    pub header_line: usize,
    pub header: Vec<String>,
    /// 各列の揃え(プレビュー描画でセル内容の左右中央寄せに使用)。
    pub aligns: Vec<Align>,
    /// 本文行: (本文行インデックス, セル群)。
    pub body: Vec<(usize, Vec<String>)>,
}

impl Table {
    /// 列数(ヘッダ基準、最低1)。
    pub fn cols(&self) -> usize {
        self.header.len().max(1)
    }
}

/// `lines[start]` からテーブルが始まるか判定し、(Table, 消費行数) を返す。
/// テーブルでなければ None。区切り行(`| --- | --- |`)の存在を必須条件とする。
pub fn detect(lines: &[&str], start: usize) -> Option<(Table, usize)> {
    let header = *lines.get(start)?;
    if !header.contains('|') || header.trim().is_empty() {
        return None;
    }
    let delim = *lines.get(start + 1)?;
    let aligns = parse_delim(delim)?; // 区切り行でなければ None

    let header_cells = split_cells(header);
    if header_cells.is_empty() {
        return None;
    }

    let mut body = Vec::new();
    let mut i = start + 2;
    while i < lines.len() {
        let l = lines[i];
        if l.trim().is_empty() || !l.contains('|') {
            break;
        }
        body.push((i, split_cells(l)));
        i += 1;
    }

    Some((
        Table {
            header_line: start,
            header: header_cells,
            aligns,
            body,
        },
        i - start,
    ))
}

/// 区切り行を解析して各列の揃えを返す。区切り行でなければ None。
fn parse_delim(line: &str) -> Option<Vec<Align>> {
    if !line.contains('-') {
        return None;
    }
    let cells = split_cells_raw(line);
    if cells.is_empty() {
        return None;
    }
    let mut aligns = Vec::with_capacity(cells.len());
    for c in &cells {
        let t = c.trim();
        if t.is_empty() {
            return None;
        }
        let left = t.starts_with(':');
        let right = t.ends_with(':');
        let core = t.trim_matches(':');
        if core.is_empty() || !core.bytes().all(|b| b == b'-') {
            return None;
        }
        aligns.push(match (left, right) {
            (true, true) => Align::Center,
            (true, false) => Align::Left,
            (false, true) => Align::Right,
            (false, false) => Align::None,
        });
    }
    Some(aligns)
}

/// 行をセルに分割(`\|` のエスケープを解除して表示用文字列に)。
pub fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);

    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&n) = chars.peek() {
                // エスケープ解除: `\|` → `|`, `\\` → `\` など
                cur.push(n);
                chars.next();
                continue;
            }
        }
        if c == '|' {
            cells.push(cur.trim().to_string());
            cur = String::new();
        } else {
            cur.push(c);
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

/// 区切り行解析用: エスケープ解除せず素朴に分割。
fn split_cells_raw(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|s| s.trim().to_string()).collect()
}

/// 指定行・列のセル内容を取得。
pub fn get_cell(text: &str, line_idx: usize, col: usize) -> Option<String> {
    let line = text.split('\n').nth(line_idx)?;
    split_cells(line).into_iter().nth(col)
}

/// 指定行の `col` 番目セルを `value` に書き換えた新しい行を返す。
/// 列数は元の行を基準とし、`value` 内の `|` と改行はエスケープ/除去する。
pub fn set_cell(line: &str, col: usize, value: &str) -> Option<String> {
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let mut cells = split_cells(line);
    if col >= cells.len() {
        return None;
    }
    let sanitized = value.replace('\n', " ").replace('|', "\\|");
    cells[col] = sanitized;
    Some(format!("{indent}| {} |", cells.join(" | ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_table() {
        let lines = vec![
            "前文",
            "| 名前 | 年齢 |",
            "| --- | ---: |",
            "| 太郎 | 30 |",
            "| 花子 | 25 |",
            "",
            "後文",
        ];
        let (t, consumed) = detect(&lines, 1).expect("table");
        assert_eq!(consumed, 4); // header + delim + 2 rows
        assert_eq!(t.header, vec!["名前", "年齢"]);
        assert_eq!(t.aligns, vec![Align::None, Align::Right]);
        assert_eq!(t.body.len(), 2);
        assert_eq!(t.body[0].1, vec!["太郎", "30"]);
    }

    #[test]
    fn not_a_table_without_delimiter() {
        let lines = vec!["| a | b |", "| c | d |"];
        assert!(detect(&lines, 0).is_none());
    }

    #[test]
    fn set_and_get_cell() {
        let line = "| 太郎 | 30 |";
        let new = set_cell(line, 1, "31").unwrap();
        assert_eq!(new, "| 太郎 | 31 |");
        assert_eq!(get_cell("a\n| 太郎 | 31 |", 1, 0).as_deref(), Some("太郎"));
    }

    #[test]
    fn escapes_pipe_in_value() {
        let line = "| a | b |";
        let new = set_cell(line, 0, "x|y").unwrap();
        assert_eq!(new, "| x\\|y | b |");
        // 書き戻したものを再解析すると元の値に戻る
        assert_eq!(split_cells(&new)[0], "x|y");
    }
}
