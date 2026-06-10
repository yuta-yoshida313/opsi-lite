// 仕様 2 / 6: シンタックスハイライト。
//
// iced の `Highlighter` トレイトを実装する。バックエンドは行単位の高速レキサで、
// Markdown の見出し・強調・コード・引用・リストマーカー・タスク、および
// Obsidian 拡張の WikiLink([[...]]) を色付けする。
//
// `Settings.revision` を編集毎にインクリメントすることで、iced 側が
// ハイライトキャッシュを破棄し先頭行から再計算する。これにより
// コードフェンス等の複数行状態も整合する。
//
// (Tree-sitter バックエンドへの差し替えは feature = "treesitter" で対応予定。
//  本レキサは増分・行単位で動作し、タイピング遅延をゼロに近づける設計。)

use std::ops::Range;

use iced::advanced::text::highlighter::{self, Highlighter};
use iced::{Color, Font};

/// ハイライト種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Heading,
    CodeFence,
    Code,
    Quote,
    ListMarker,
    TaskOpen,
    TaskDone,
    WikiLink,
    Bold,
    Italic,
}

impl Style {
    /// ダークテーマ向けの配色。
    pub fn color(self) -> Color {
        match self {
            Style::Heading => Color::from_rgb(0.45, 0.74, 1.0), // 明るい青
            Style::CodeFence => Color::from_rgb(0.55, 0.55, 0.60),
            Style::Code => Color::from_rgb(0.90, 0.62, 0.42), // オレンジ
            Style::Quote => Color::from_rgb(0.55, 0.78, 0.55), // 緑
            Style::ListMarker => Color::from_rgb(0.75, 0.55, 0.95), // 紫
            Style::TaskOpen => Color::from_rgb(0.85, 0.75, 0.35), // 黄
            Style::TaskDone => Color::from_rgb(0.45, 0.65, 0.45), // くすんだ緑
            Style::WikiLink => Color::from_rgb(0.40, 0.80, 0.85), // シアン
            Style::Bold => Color::from_rgb(0.95, 0.90, 0.70),
            Style::Italic => Color::from_rgb(0.80, 0.85, 0.95),
        }
    }
}

/// ハイライタ設定。`revision` が変わると全体が再計算される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub revision: u64,
}

pub struct MarkdownHighlighter {
    in_code_fence: bool,
    current_line: usize,
}

impl Highlighter for MarkdownHighlighter {
    type Settings = Settings;
    type Highlight = Style;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Style)>;

    fn new(_settings: &Self::Settings) -> Self {
        Self {
            in_code_fence: false,
            current_line: 0,
        }
    }

    fn update(&mut self, _new_settings: &Self::Settings) {
        // revision 変更時: 先頭行から再計算するため状態をリセット。
        self.in_code_fence = false;
        self.current_line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
        if line == 0 {
            self.in_code_fence = false;
        }
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let spans = lex_line(line, &mut self.in_code_fence);
        self.current_line += 1;
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

/// to_format ヘルパ: Style → iced の描画フォーマット。
pub fn to_format(style: &Style, _theme: &iced::Theme) -> highlighter::Format<Font> {
    let mut font = None;
    if matches!(style, Style::Bold | Style::Heading) {
        font = Some(Font {
            weight: iced::font::Weight::Bold,
            ..Font::MONOSPACE
        });
    } else if matches!(style, Style::Italic) {
        font = Some(Font {
            style: iced::font::Style::Italic,
            ..Font::MONOSPACE
        });
    }
    highlighter::Format {
        color: Some(style.color()),
        font,
    }
}

/// 1 行を解析し、(バイト範囲, Style) のリストを返す。
fn lex_line(line: &str, in_code_fence: &mut bool) -> Vec<(Range<usize>, Style)> {
    let mut out = Vec::new();
    let trimmed_start = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    // --- コードフェンス ``` / ~~~ ---
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        *in_code_fence = !*in_code_fence;
        out.push((0..line.len(), Style::CodeFence));
        return out;
    }
    if *in_code_fence {
        out.push((0..line.len(), Style::Code));
        return out;
    }

    // --- 見出し # ---
    if trimmed.starts_with('#') {
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if hashes <= 6
            && trimmed[hashes..]
                .chars()
                .next()
                .map(|c| c == ' ')
                .unwrap_or(trimmed.len() == hashes)
        {
            out.push((0..line.len(), Style::Heading));
            return out;
        }
    }

    // --- 引用 > ---
    if trimmed.starts_with('>') {
        out.push((0..line.len(), Style::Quote));
        return out;
    }

    // --- タスク / リストマーカー ---
    if let Some(state) = crate::tasks::task_state(line) {
        // "- [ ]" / "- [x]" の checkbox 部分までを着色
        if let Some(close) = line.find(']') {
            let style = if state { Style::TaskDone } else { Style::TaskOpen };
            out.push((trimmed_start..close + 1, style));
        }
    } else if is_list_marker(trimmed) {
        // リストマーカー1〜2文字を着色
        let marker_len = list_marker_len(trimmed);
        out.push((trimmed_start..trimmed_start + marker_len, Style::ListMarker));
    }

    // --- インライン要素(WikiLink / コード / 強調) ---
    inline_spans(line, &mut out);

    // 範囲開始位置でソート(iced は順序付きを期待)
    out.sort_by_key(|(r, _)| r.start);
    dedup_overlaps(&mut out);
    out
}

fn is_list_marker(trimmed: &str) -> bool {
    let b = trimmed.as_bytes();
    if b.is_empty() {
        return false;
    }
    (b[0] == b'-' || b[0] == b'*' || b[0] == b'+') && b.get(1) == Some(&b' ')
}

fn list_marker_len(trimmed: &str) -> usize {
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        1
    } else {
        0
    }
}

/// インライン: WikiLink・`code`・**bold**・*italic* のバイト範囲を抽出。
fn inline_spans(line: &str, out: &mut Vec<(Range<usize>, Style)>) {
    // WikiLink
    for link in crate::wikilink::find_all(line) {
        out.push((link.range, Style::WikiLink));
    }

    let bytes = line.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        match bytes[i] {
            b'`' => {
                if let Some(rel) = find_byte(&line[i + 1..], b'`') {
                    let end = i + 1 + rel + 1;
                    out.push((i..end, Style::Code));
                    i = end;
                    continue;
                }
            }
            b'*' => {
                // ** bold **
                if bytes.get(i + 1) == Some(&b'*') {
                    if let Some(rel) = line[i + 2..].find("**") {
                        let end = i + 2 + rel + 2;
                        out.push((i..end, Style::Bold));
                        i = end;
                        continue;
                    }
                } else if let Some(rel) = find_byte(&line[i + 1..], b'*') {
                    // * italic *
                    let end = i + 1 + rel + 1;
                    out.push((i..end, Style::Italic));
                    i = end;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

fn find_byte(s: &str, target: u8) -> Option<usize> {
    s.as_bytes().iter().position(|&b| b == target)
}

/// 重なり合う範囲を除去(先勝ち)。WikiLink を優先したいので安定ソート後に処理。
fn dedup_overlaps(spans: &mut Vec<(Range<usize>, Style)>) {
    let mut result: Vec<(Range<usize>, Style)> = Vec::with_capacity(spans.len());
    let mut last_end = 0usize;
    for (range, style) in spans.drain(..) {
        if range.start >= last_end {
            last_end = range.end;
            result.push((range, style));
        }
        // 重なる場合はスキップ(既存を優先)
    }
    *spans = result;
}
