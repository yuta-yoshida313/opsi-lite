// プレビュー兼エディタ描画。
//
// 既定の主画面。各ブロックをクリックすると、その行が text_input に変わり
// 生Markdownをその場で編集できる(Obsidian の Live Preview 方式)。
//   - 段落/見出し/引用/箇条書き: クリックで行編集(mouse_area)
//   - タスク: チェックボックスはトグル、ラベルはクリックで行編集
//   - テーブル: セルをクリックでセル編集
//   - WikiLink: リンク部分のクリックはジャンプ、それ以外は行編集
//   - コードブロック: クリックでコード編集モード(Source)へ

use iced::alignment::Horizontal;
use iced::widget::{
    button, checkbox, container, mouse_area, row, text, text_input, Column, Row, Space,
};
use iced::{Border, Element, Font, Length, Theme};

use crate::app::{Message, ViewMode};
use crate::table;
use crate::wikilink;

/// 編集中の text_input に付与する固定 Id(常に高々1つ)。
pub const ACTIVE_INPUT_ID: &str = "opsi-active-input";

fn link_target(inner: &str) -> String {
    inner.split('|').next().unwrap_or(inner).trim().to_string()
}

/// 本文をプレビュー(兼インラインエディタ)として描画する。
pub fn view<'a>(
    source: &'a str,
    editing_line: Option<usize>,
    editing_cell: Option<(usize, usize)>,
    edit_buffer: &'a str,
    font: Font,
) -> Element<'a, Message> {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut col = Column::new().spacing(4).padding(16).width(Length::Fill);
    let mut in_fence = false;
    let mut fence_start = 0usize;
    let mut fence_buf: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // コードフェンス(クリックで Source モードへ)
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if in_fence {
                col = col.push(code_block(&fence_buf.join("\n"), font, fence_start));
                fence_buf.clear();
            } else {
                fence_start = i;
            }
            in_fence = !in_fence;
            i += 1;
            continue;
        }
        if in_fence {
            fence_buf.push(line.to_string());
            i += 1;
            continue;
        }

        // 行編集中(この行が text_input)
        if editing_line == Some(i) {
            col = col.push(active_input(edit_buffer, i, font));
            i += 1;
            continue;
        }

        // テーブル(複数行・セル編集)
        if let Some((tbl, consumed)) = table::detect(&lines, i) {
            col = col.push(table_view(&tbl, editing_cell, edit_buffer, font));
            i += consumed;
            continue;
        }

        // タスク行
        if let Some(checked) = crate::tasks::task_state(line) {
            col = col.push(task_row(i, checked, line, font));
            i += 1;
            continue;
        }

        // 見出し
        if let Some((level, body)) = heading(trimmed) {
            let size = match level {
                1 => 28.0,
                2 => 23.0,
                3 => 19.0,
                _ => 16.0,
            };
            let f = Font {
                weight: iced::font::Weight::Bold,
                ..font
            };
            col = col.push(clickable_line(i, text(body.to_string()).size(size).font(f)));
            i += 1;
            continue;
        }

        // 引用
        if let Some(rest) = trimmed
            .strip_prefix("> ")
            .or_else(|| trimmed.strip_prefix('>'))
        {
            let inner = container(inline_row(rest, 16.0, font))
                .padding([2, 10])
                .width(Length::Fill);
            col = col.push(clickable_line(i, inner));
            i += 1;
            continue;
        }

        // 箇条書き
        if let Some(rest) = strip_bullet(trimmed) {
            let r = row![
                text("•  ").size(16).font(font),
                inline_row(rest, 16.0, font)
            ]
            .spacing(2);
            col = col.push(clickable_line(i, r));
            i += 1;
            continue;
        }

        // 空行(クリック可能な薄い領域)
        if line.trim().is_empty() {
            col = col.push(clickable_line(
                i,
                container(Space::new().height(Length::Fixed(10.0))).width(Length::Fill),
            ));
            i += 1;
            continue;
        }

        // 段落
        col = col.push(clickable_line(i, inline_row(line, 16.0, font)));
        i += 1;
    }

    if !fence_buf.is_empty() {
        col = col.push(code_block(&fence_buf.join("\n"), font, fence_start));
    }

    col.into()
}

/// 任意の描画内容を「クリックで行編集」可能にするラッパ。
fn clickable_line<'a>(
    line_idx: usize,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    mouse_area(container(content).width(Length::Fill))
        .on_press(Message::LineFocus(line_idx))
        .into()
}

/// 行編集用の text_input。
fn active_input<'a>(buffer: &'a str, line_idx: usize, font: Font) -> Element<'a, Message> {
    text_input("", buffer)
        .id(ACTIVE_INPUT_ID)
        .on_input(Message::LineInput)
        .on_submit(Message::LineCommit(line_idx))
        .font(font)
        .size(16)
        .padding(6)
        .width(Length::Fill)
        .into()
}

fn heading(trimmed: &str) -> Option<(usize, &str)> {
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    Some((level, trimmed[level..].trim_start()))
}

fn strip_bullet(trimmed: &str) -> Option<&str> {
    for m in ["- ", "* ", "+ "] {
        if let Some(r) = trimmed.strip_prefix(m) {
            return Some(r);
        }
    }
    None
}

/// コードブロック(クリックで Source モードへ切替)。
fn code_block(code: &str, _font: Font, _start: usize) -> Element<'static, Message> {
    mouse_area(
        container(text(code.to_string()).font(Font::MONOSPACE).size(14))
            .padding(8)
            .width(Length::Fill)
            .style(container::dark),
    )
    .on_press(Message::SwitchMode(ViewMode::Source))
    .into()
}

/// タスク行: チェックボックスでトグル、ラベルクリックで行編集。
fn task_row(line_idx: usize, checked: bool, line: &str, font: Font) -> Element<'static, Message> {
    let label = line
        .find(']')
        .map(|i| line[i + 1..].trim_start().to_string())
        .unwrap_or_default();
    let cb = checkbox(checked).on_toggle(move |_| Message::PreviewToggleTask(line_idx));
    let label_el = mouse_area(container(text(label).size(16).font(font)).width(Length::Fill))
        .on_press(Message::LineFocus(line_idx));
    Row::new().spacing(6).push(cb).push(label_el).into()
}

/// テーブルを罫線付きグリッドとして描画。
/// ヘッダは背景強調＋太字、各列は区切り行(`:---:` 等)の指定に従って左右中央揃え。
fn table_view<'a>(
    t: &table::Table,
    editing_cell: Option<(usize, usize)>,
    edit_buffer: &'a str,
    font: Font,
) -> Element<'a, Message> {
    let cols = t.cols();
    // spacing(0) でセル同士を密着させ、各セルの 1px ボーダーが格子線になる。
    let mut grid = Column::new().spacing(0);
    grid = grid.push(cell_row(
        t.header_line,
        &t.header,
        cols,
        &t.aligns,
        editing_cell,
        edit_buffer,
        true,
        font,
    ));
    for (line_idx, cells) in &t.body {
        grid = grid.push(cell_row(
            *line_idx,
            cells,
            cols,
            &t.aligns,
            editing_cell,
            edit_buffer,
            false,
            font,
        ));
    }
    container(grid).padding([8, 0]).width(Length::Fill).into()
}

fn cell_row<'a>(
    line_idx: usize,
    cells: &[String],
    cols: usize,
    aligns: &[table::Align],
    editing_cell: Option<(usize, usize)>,
    edit_buffer: &'a str,
    header: bool,
    font: Font,
) -> Element<'a, Message> {
    let mut r = Row::new().spacing(0);
    for col in 0..cols {
        let value = cells.get(col).cloned().unwrap_or_default();
        let is_editing = editing_cell == Some((line_idx, col));
        let align = aligns.get(col).copied().unwrap_or(table::Align::None);
        let inner: Element<'a, Message> = if is_editing {
            text_input("", edit_buffer)
                .id(ACTIVE_INPUT_ID)
                .on_input(Message::TableCellInput)
                .on_submit(Message::TableCellCommit {
                    line: line_idx,
                    col,
                })
                .font(font)
                .padding(0)
                .size(14)
                .width(Length::Fill)
                .into()
        } else {
            // セル内容を行内描画(WikiLink はクリックでジャンプ)。
            // リンク以外の余白をクリックするとセル編集に入る。
            mouse_area(
                container(cell_inline(&value, header, font))
                    .width(Length::Fill)
                    .align_x(align_x(align)),
            )
            .on_press(Message::TableCellFocus {
                line: line_idx,
                col,
            })
            .into()
        };
        r = r.push(
            container(inner)
                .padding([5, 8])
                .width(Length::FillPortion(1))
                .style(cell_style(header)),
        );
    }
    r.into()
}

/// セルの表示用行内要素(空セルでも高さを確保)。ヘッダは太字。
fn cell_inline(value: &str, header: bool, font: Font) -> Element<'static, Message> {
    let f = if header {
        Font {
            weight: iced::font::Weight::Bold,
            ..font
        }
    } else {
        font
    };
    let shown = if value.trim().is_empty() { " " } else { value };
    inline_row(shown, 14.0, f)
}

/// 区切り行(`:---`, `---:`, `:---:`)で指定された列揃えを描画用に変換。
fn align_x(a: table::Align) -> Horizontal {
    match a {
        table::Align::Right => Horizontal::Right,
        table::Align::Center => Horizontal::Center,
        _ => Horizontal::Left,
    }
}

/// テーブルセルの罫線・背景スタイル。ヘッダは背景を一段強調する。
fn cell_style(header: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let palette = theme.extended_palette();
        let background = if header {
            palette.background.weak.color
        } else {
            palette.background.base.color
        };
        container::Style {
            background: Some(background.into()),
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        }
    }
}

/// 行内の WikiLink をボタン化しつつテキストを描画(リンクはジャンプ)。
fn inline_row(line: &str, size: f32, font: Font) -> Element<'static, Message> {
    let links = wikilink::find_all(line);
    if links.is_empty() {
        return text(line.to_string()).size(size).font(font).into();
    }
    let mut r = Row::new().spacing(0);
    let mut last = 0usize;
    for l in links {
        if l.range.start > last {
            r = r.push(
                text(line[last..l.range.start].to_string())
                    .size(size)
                    .font(font),
            );
        }
        let disp = l.display().to_string();
        let target = link_target(&l.inner);
        r = r.push(
            button(text(disp).size(size).font(font))
                .padding([0, 2])
                .on_press(Message::PreviewFollowLink(target))
                .style(button::text),
        );
        last = l.range.end;
    }
    if last < line.len() {
        r = r.push(text(line[last..].to_string()).size(size).font(font));
    }
    r.into()
}
