// 仕様 4.2 WikiLink ([[...]]) の検知ロジック。
// テキスト内の [[...]] 範囲・カーソル直下のリンク抽出・補完トリガ検出を担う。

use std::ops::Range;

/// テキスト中の 1 つの WikiLink。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// `[[` の `[` から `]]` の `]` までのバイト範囲(行内 or 全文に応じる)。
    pub range: Range<usize>,
    /// 内側の生テキスト(`名前|表示名` 等を含む)。
    pub inner: String,
}

impl WikiLink {
    /// 表示名(`|` の右側があればそれ、なければ名前部分)。
    pub fn display(&self) -> &str {
        if let Some(idx) = self.inner.find('|') {
            self.inner[idx + 1..].trim()
        } else {
            self.inner.trim()
        }
    }
}

/// 与えられた文字列(1 行 or 全文)に含まれる全 WikiLink を返す。
/// 範囲は引数文字列先頭からのバイトオフセット。
pub fn find_all(text: &str) -> Vec<WikiLink> {
    let bytes = text.as_bytes();
    let mut links = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            // 閉じ "]]" を探す(同一行・改行をまたがない)
            if let Some(end_rel) = find_close(&text[i + 2..]) {
                let inner_start = i + 2;
                let inner_end = inner_start + end_rel;
                let inner = text[inner_start..inner_end].to_string();
                if !inner.is_empty() {
                    links.push(WikiLink {
                        range: i..inner_end + 2,
                        inner,
                    });
                }
                i = inner_end + 2;
                continue;
            }
        }
        // 次の文字境界へ
        i += 1;
    }
    links
}

/// `]]` または改行までの相対位置(内側終端)を返す。改行が先に来たら None。
fn find_close(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'\n' {
            return None;
        }
        if b[i] == b']' && b[i + 1] == b']' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// カーソル位置(行テキストとバイト列、行内のカラム=文字単位)直下にある WikiLink を返す。
/// 仕様 4.2 リンクジャンプの「[[ファイル名]] の上で」判定に使用。
pub fn link_at_column(line: &str, char_col: usize) -> Option<WikiLink> {
    // 文字カラム → バイトオフセットへ変換
    let byte_col = line
        .char_indices()
        .nth(char_col)
        .map(|(b, _)| b)
        .unwrap_or(line.len());
    find_all(line)
        .into_iter()
        .find(|l| l.range.contains(&byte_col) || l.range.end == byte_col || l.range.start == byte_col)
}

/// 仕様 4.2 自動補完トリガ:
/// カーソル左側を見て、未閉鎖の `[[` があればその後ろに打たれたクエリ文字列を返す。
/// 例: `... see [[Foo Ba|cursor` → Some("Foo Ba")。閉じ `]]` 後や `[[` が無ければ None。
pub fn active_completion_query(line_before_cursor: &str) -> Option<String> {
    // 直近の "[[" を探す
    let open = line_before_cursor.rfind("[[")?;
    let after = &line_before_cursor[open + 2..];
    // 既に閉じている、または別の "[[" / "]]" を挟むなら補完対象外
    if after.contains("]]") || after.contains('[') || after.contains('\n') {
        return None;
    }
    Some(after.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_basic_links() {
        let links = find_all("foo [[Bar]] baz [[Qux|Q]]");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].inner, "Bar");
        assert_eq!(links[1].display(), "Q");
    }

    #[test]
    fn completion_query_detection() {
        assert_eq!(active_completion_query("see [[Fo"), Some("Fo".to_string()));
        assert_eq!(active_completion_query("see [[Foo]] done"), None);
        assert_eq!(active_completion_query("no link here"), None);
    }

    #[test]
    fn no_link_across_newline() {
        assert!(find_all("[[unclosed\nnext").is_empty());
    }
}
