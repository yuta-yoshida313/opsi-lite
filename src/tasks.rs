// 仕様 4.3 インタラクティブ・タスクリスト。
// Markdown 標準のタスク記法 `- [ ]` / `- [x]` の検出とトグル(状態反転)。

/// 行がタスク行であれば、(インデント+リストマーカー長, 現在チェック済みか) を返す。
/// 対応マーカー: `-`, `*`, `+`、および順序付きリスト `1.` も許容。
pub fn task_state(line: &str) -> Option<bool> {
    parse_task(line).map(|t| t.checked)
}

struct Task {
    /// `[` の直前(チェック文字 `[?]` の `?` )のバイト位置
    mark_byte: usize,
    checked: bool,
}

fn parse_task(line: &str) -> Option<Task> {
    let bytes = line.as_bytes();
    let mut i = 0;
    // 先頭の空白
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    // リストマーカー: "- " "* " "+ " もしくは "<digits>. "
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'*' || bytes[i] == b'+') {
        i += 1;
    } else {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > start && i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') {
            i += 1;
        } else {
            return None;
        }
    }
    // マーカー直後の空白(1つ以上)
    if i >= bytes.len() || (bytes[i] != b' ' && bytes[i] != b'\t') {
        return None;
    }
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    // チェックボックス "[ ]" / "[x]" / "[X]"
    if i + 2 < bytes.len() && bytes[i] == b'[' && bytes[i + 2] == b']' {
        let c = bytes[i + 1];
        let checked = match c {
            b' ' => false,
            b'x' | b'X' => true,
            _ => return None,
        };
        return Some(Task {
            mark_byte: i + 1,
            checked,
        });
    }
    None
}

/// 1 行をトグルした結果の新しい行を返す。タスク行でなければ None。
pub fn toggle_line(line: &str) -> Option<String> {
    let task = parse_task(line)?;
    let mut s = line.to_string();
    let replacement = if task.checked { ' ' } else { 'x' };
    // mark_byte は ASCII の `[` 直後なので 1 バイト置換で安全
    s.replace_range(task.mark_byte..task.mark_byte + 1, &replacement.to_string());
    Some(s)
}

/// 全文と行番号(0始まり)を受け取り、その行をトグルした全文を返す。
/// 変更が無ければ None。
pub fn toggle_in_text(text: &str, line_idx: usize) -> Option<String> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let target = *lines.get(line_idx)?;
    let toggled = toggle_line(target)?;
    let owned = toggled;
    lines[line_idx] = owned.as_str();
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_and_toggle() {
        assert_eq!(task_state("- [ ] todo"), Some(false));
        assert_eq!(task_state("- [x] done"), Some(true));
        assert_eq!(task_state("  * [X] nested"), Some(true));
        assert_eq!(task_state("plain text"), None);
        assert_eq!(task_state("- bullet"), None);

        assert_eq!(toggle_line("- [ ] todo").as_deref(), Some("- [x] todo"));
        assert_eq!(toggle_line("- [x] done").as_deref(), Some("- [ ] done"));
        assert_eq!(toggle_line("  - [ ] indented").as_deref(), Some("  - [x] indented"));
    }

    #[test]
    fn toggle_within_text() {
        let t = "line0\n- [ ] task\nline2";
        assert_eq!(
            toggle_in_text(t, 1).as_deref(),
            Some("line0\n- [x] task\nline2")
        );
        assert_eq!(toggle_in_text(t, 0), None);
    }
}
