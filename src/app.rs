// Opsi-Lite アプリケーション本体(iced)。
//
// 既定はプレビュー画面で、各ブロックをクリックするとその場で編集できる
// (Obsidian の Live Preview 方式)。右上ボタンで生Markdownの Source 編集や
// 設定画面(テーマ・フォント)へ切り替える。

use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{
    button, column, container, operation, pick_list, row, scrollable, text, text_editor, Column,
    Space,
};
use iced::{keyboard, Element, Font, Length, Subscription, Task, Theme};

use crate::highlight::{self, MarkdownHighlighter};
use crate::index::{self, Index};
use crate::{config, preview, table, tasks, wikilink};

const WELCOME: &str = "# Opsi-Lite\n\nObsidian互換・超軽量Markdownエディタ\n\n\
プレビュー上の行をクリックすると、その場で編集できます。\n\n\
- [ ] チェックボックスはクリックでトグル\n\
- [ ] `[[` でWikiLink、`F12`でジャンプ\n\
- [ ] 右上「</> コード」で生Markdown編集、「⚙ 設定」でテーマ/フォント変更\n";

/// ライセンス本文(配布物に含める / 設定画面で表示)。
const LICENSE_TEXT: &str = include_str!("../LICENSE");

/// 寄付・リポジトリのURL(設定画面のリンクから開く)。
const DONATE_URL: &str = "https://ko-fi.com/yoshidasoftware";
const REPO_URL: &str = "https://github.com/yuta-yoshida313/opsi-lite";

/// 設定で選べるフォント(いずれも &'static str = iced の Font 名に使用)。
pub const FONT_NAMES: &[&str] = &[
    "Yu Gothic UI",
    "Meiryo",
    "MS Gothic",
    "BIZ UDGothic",
    "BIZ UDPGothic",
    "Yu Mincho",
    "Consolas",
    "Cascadia Code",
    "Cascadia Mono",
];

/// 画面モード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Preview,
    Source,
    Settings,
}

/// 現在のインライン編集対象。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    None,
    Line(usize),
    Cell { line: usize, col: usize },
}

pub struct App {
    content: text_editor::Content,
    path: Option<PathBuf>,
    temp_root: Option<PathBuf>,
    index: Index,
    dirty: bool,
    revision: u64,
    history: Vec<PathBuf>,
    suggestions: Vec<String>,
    suggestion_query: Option<String>,
    status: String,
    /// プレビュー描画が借用する本文キャッシュ。
    text_cache: String,
    /// 画面モード(既定: プレビュー)。
    view_mode: ViewMode,
    /// 設定画面から戻る先。
    prev_mode: ViewMode,
    /// インライン編集対象と入力バッファ。
    edit: Edit,
    edit_buffer: String,
    /// 設定: テーマ・フォント。
    theme: Theme,
    content_font: Font,
    font_name: &'static str,
}

#[derive(Debug, Clone)]
pub enum Message {
    Edit(text_editor::Action),
    Indexed(Index),
    Save,
    Saved(Result<PathBuf, String>),
    FollowLinkUnderCursor,
    Back,
    Loaded(PathBuf, Result<String, String>),
    ToggleTaskAtCursor,
    PreviewToggleTask(usize),
    PreviewFollowLink(String),
    CreateAndOpen(PathBuf),
    SuggestionChosen(String),
    DismissSuggestions,
    // 画面モード
    SwitchMode(ViewMode),
    OpenSettings,
    ToggleSourcePreview,
    // 設定
    ThemeSelected(Theme),
    FontSelected(&'static str),
    OpenUrl(&'static str),
    // インライン行編集(プレビュー)
    LineFocus(usize),
    LineInput(String),
    LineCommit(usize),
    // テーブル編集
    TableCellFocus { line: usize, col: usize },
    TableCellInput(String),
    TableCellCommit { line: usize, col: usize },
    InsertTable,
    Noop,
}

impl App {
    pub fn new(arg: Option<PathBuf>) -> (Self, Task<Message>) {
        // 設定読込
        let cfg = config::load();
        let theme = cfg
            .theme
            .as_deref()
            .map(theme_from_name)
            .unwrap_or(Theme::TokyoNight);
        let (content_font, font_name) = font_from_name(cfg.font.as_deref().unwrap_or(FONT_NAMES[0]));

        let mut app = App {
            content: text_editor::Content::new(),
            path: None,
            temp_root: None,
            index: Index::default(),
            dirty: false,
            revision: 0,
            history: Vec::new(),
            suggestions: Vec::new(),
            suggestion_query: None,
            status: String::new(),
            text_cache: String::new(),
            view_mode: ViewMode::Preview,
            prev_mode: ViewMode::Preview,
            edit: Edit::None,
            edit_buffer: String::new(),
            theme,
            content_font,
            font_name,
        };

        let mut task = Task::none();

        match arg {
            Some(p) => {
                let p = std::fs::canonicalize(&p).unwrap_or(p);
                if p.is_dir() {
                    app.temp_root = Some(p.clone());
                    app.content = text_editor::Content::with_text(WELCOME);
                    app.status = format!("フォルダ: {}", p.display());
                    task = scan_task(p);
                } else {
                    match std::fs::read_to_string(&p) {
                        Ok(s) => app.content = text_editor::Content::with_text(&s),
                        Err(e) => app.status = format!("読込失敗: {e}"),
                    }
                    let root = p.parent().map(|d| d.to_path_buf());
                    app.temp_root = root.clone();
                    app.path = Some(p);
                    if let Some(root) = root {
                        task = scan_task(root);
                    }
                }
            }
            None => {
                app.content = text_editor::Content::with_text(WELCOME);
                app.status = "新規バッファ".into();
            }
        }

        app.text_cache = app.content.text();
        (app, task)
    }

    pub fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("untitled");
        let mark = if self.dirty { "● " } else { "" };
        format!("{mark}{name} — Opsi-Lite")
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        self.text_cache = self.content.text();
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                let is_edit = action.is_edit();
                self.content.perform(action);
                if is_edit {
                    self.dirty = true;
                    self.revision += 1;
                    self.refresh_suggestions();
                }
                Task::none()
            }
            Message::Indexed(idx) => {
                let n = idx.all_stems().len();
                self.index = idx;
                if self.status.is_empty() || self.status.starts_with("フォルダ") {
                    self.status = format!("索引構築完了: {n} ノート");
                }
                Task::none()
            }
            Message::Save => self.save(),
            Message::Saved(res) => {
                match res {
                    Ok(p) => {
                        self.dirty = false;
                        self.status = format!("保存しました: {}", p.display());
                    }
                    Err(e) => self.status = format!("保存失敗: {e}"),
                }
                Task::none()
            }
            Message::ToggleTaskAtCursor => {
                let (line, _) = self.cursor_lc();
                self.toggle_task(line);
                Task::none()
            }
            Message::PreviewToggleTask(line) => {
                self.toggle_task(line);
                self.save_silent()
            }
            Message::FollowLinkUnderCursor => self.follow_link_under_cursor(),
            Message::PreviewFollowLink(name) => self.follow_link(name),
            Message::CreateAndOpen(path) => {
                if !path.exists() {
                    let _ = std::fs::write(&path, "");
                }
                self.open_path(path)
            }
            Message::Loaded(path, res) => {
                match res {
                    Ok(s) => {
                        self.content = text_editor::Content::with_text(&s);
                        self.path = Some(path);
                        self.dirty = false;
                        self.revision += 1;
                        self.edit = Edit::None;
                        self.suggestions.clear();
                        self.suggestion_query = None;
                        self.status = "読込完了".into();
                    }
                    Err(e) => self.status = format!("読込失敗: {e}"),
                }
                Task::none()
            }
            Message::Back => {
                if let Some(prev) = self.history.pop() {
                    return Task::perform(load(prev.clone()), move |r| {
                        Message::Loaded(prev.clone(), r)
                    });
                }
                self.status = "履歴がありません".into();
                Task::none()
            }
            Message::SuggestionChosen(name) => self.choose_suggestion(name),
            Message::DismissSuggestions => {
                self.suggestions.clear();
                self.suggestion_query = None;
                self.edit = Edit::None;
                Task::none()
            }
            Message::SwitchMode(mode) => {
                if mode == ViewMode::Settings {
                    self.prev_mode = self.view_mode;
                }
                if mode != ViewMode::Preview {
                    self.edit = Edit::None;
                }
                self.suggestions.clear();
                self.view_mode = mode;
                Task::none()
            }
            Message::OpenSettings => {
                self.prev_mode = self.view_mode;
                self.view_mode = ViewMode::Settings;
                self.edit = Edit::None;
                Task::none()
            }
            Message::ToggleSourcePreview => {
                self.view_mode = match self.view_mode {
                    ViewMode::Source => ViewMode::Preview,
                    ViewMode::Preview => ViewMode::Source,
                    ViewMode::Settings => self.prev_mode,
                };
                self.edit = Edit::None;
                Task::none()
            }
            Message::ThemeSelected(t) => {
                self.theme = t;
                config::save(&self.theme.to_string(), self.font_name);
                self.status = "テーマを変更しました".into();
                Task::none()
            }
            Message::FontSelected(name) => {
                let (font, n) = font_from_name(name);
                self.content_font = font;
                self.font_name = n;
                config::save(&self.theme.to_string(), self.font_name);
                self.status = format!("フォントを変更しました: {n}");
                Task::none()
            }
            Message::OpenUrl(url) => {
                // 既定ブラウザでURLを開く(UIをブロックしないよう別スレッド)。
                std::thread::spawn(move || {
                    let _ = open::that(url);
                });
                Task::none()
            }
            Message::LineFocus(idx) => {
                self.edit = Edit::Line(idx);
                self.edit_buffer = self
                    .text_cache
                    .split('\n')
                    .nth(idx)
                    .unwrap_or("")
                    .to_string();
                self.suggestions.clear();
                self.suggestion_query = None;
                operation::focus(preview::ACTIVE_INPUT_ID)
            }
            Message::LineInput(s) => {
                if let Edit::Line(idx) = self.edit {
                    self.edit_buffer = s.clone();
                    self.replace_line(idx, &s);
                    self.refresh_suggestions_from(&s);
                }
                Task::none()
            }
            Message::LineCommit(idx) => {
                // Enter で次の行を新規作成して編集継続。
                self.insert_line_after(idx);
                self.edit = Edit::Line(idx + 1);
                self.edit_buffer = String::new();
                self.suggestions.clear();
                self.suggestion_query = None;
                operation::focus(preview::ACTIVE_INPUT_ID)
            }
            Message::TableCellFocus { line, col } => {
                self.edit = Edit::Cell { line, col };
                self.edit_buffer = table::get_cell(&self.text_cache, line, col).unwrap_or_default();
                operation::focus(preview::ACTIVE_INPUT_ID)
            }
            Message::TableCellInput(value) => {
                if let Edit::Cell { line, col } = self.edit {
                    self.edit_buffer = value.clone();
                    self.write_table_cell(line, col, &value);
                }
                Task::none()
            }
            Message::TableCellCommit { line, col } => {
                let next = col + 1;
                match table::get_cell(&self.text_cache, line, next) {
                    Some(v) => {
                        self.edit = Edit::Cell { line, col: next };
                        self.edit_buffer = v;
                        operation::focus(preview::ACTIVE_INPUT_ID)
                    }
                    None => {
                        self.edit = Edit::None;
                        Task::none()
                    }
                }
            }
            Message::InsertTable => {
                let template = "\n| 列1 | 列2 | 列3 |\n| --- | --- | --- |\n|  |  |  |\n";
                if self.view_mode == ViewMode::Source {
                    use text_editor::{Action, Edit as TEdit};
                    self.content
                        .perform(Action::Edit(TEdit::Paste(Arc::new(template.to_string()))));
                } else {
                    // プレビュー時は末尾に追記。
                    let mut t = self.text_cache.clone();
                    t.push_str(template);
                    self.set_content(&t);
                }
                self.dirty = true;
                self.revision += 1;
                self.status = "テーブルを挿入しました".into();
                Task::none()
            }
            Message::Noop => Task::none(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _status, _window| {
            use keyboard::key::Named;
            use keyboard::{Event::KeyPressed, Key};
            if let iced::Event::Keyboard(KeyPressed { key, modifiers, .. }) = event {
                match key {
                    Key::Named(Named::Enter) if modifiers.command() => {
                        Some(Message::ToggleTaskAtCursor)
                    }
                    Key::Named(Named::F12) => Some(Message::FollowLinkUnderCursor),
                    Key::Named(Named::ArrowLeft) if modifiers.alt() => Some(Message::Back),
                    Key::Named(Named::Escape) => Some(Message::DismissSuggestions),
                    Key::Character(c) if modifiers.command() && c.as_str() == "s" => {
                        Some(Message::Save)
                    }
                    Key::Character(c) if modifiers.command() && c.as_str() == "e" => {
                        Some(Message::ToggleSourcePreview)
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    // ================= ビュー =================

    pub fn view(&self) -> Element<'_, Message> {
        match self.view_mode {
            ViewMode::Preview => self.preview_view(),
            ViewMode::Source => self.source_view(),
            ViewMode::Settings => self.settings_view(),
        }
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let path_label = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(未保存)".to_string());
        container(
            row![
                text(format!("{}{}", if self.dirty { "● " } else { "" }, path_label)).size(12),
                Space::new().width(Length::Fill),
                text(&self.status).size(12),
            ]
            .spacing(8),
        )
        .padding([4, 10])
        .width(Length::Fill)
        .into()
    }

    fn preview_view(&self) -> Element<'_, Message> {
        let toolbar = row![
            tb_button("保存", Message::Save),
            tb_button("← 戻る", Message::Back),
            tb_button("表を挿入", Message::InsertTable),
            Space::new().width(Length::Fill),
            tb_button("</> コード", Message::SwitchMode(ViewMode::Source)),
            tb_button("⚙ 設定", Message::OpenSettings),
        ]
        .spacing(8)
        .padding(8)
        .align_y(iced::Alignment::Center);

        let (editing_line, editing_cell) = self.edit_targets();
        let body = scrollable(preview::view(
            &self.text_cache,
            editing_line,
            editing_cell,
            &self.edit_buffer,
            self.content_font,
        ))
        .height(Length::Fill);

        let mut main = Column::new()
            .push(container(body).width(Length::Fill).height(Length::Fill));
        if !self.suggestions.is_empty() {
            main = main.push(self.suggestion_panel());
        }

        column![toolbar, main, self.status_bar()]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn source_view(&self) -> Element<'_, Message> {
        let toolbar = row![
            tb_button("保存", Message::Save),
            tb_button("← 戻る", Message::Back),
            Space::new().width(Length::Fill),
            tb_button("👁 プレビュー", Message::SwitchMode(ViewMode::Preview)),
            tb_button("⚙ 設定", Message::OpenSettings),
        ]
        .spacing(8)
        .padding(8)
        .align_y(iced::Alignment::Center);

        let editor = text_editor(&self.content)
            .on_action(Message::Edit)
            .font(self.content_font)
            .size(15)
            .height(Length::Fill)
            .highlight_with::<MarkdownHighlighter>(
                highlight::Settings {
                    revision: self.revision,
                },
                highlight::to_format,
            );

        let mut main = Column::new().push(editor);
        if !self.suggestions.is_empty() {
            main = main.push(self.suggestion_panel());
        }

        column![
            toolbar,
            container(main).width(Length::Fill).height(Length::Fill).padding(4),
            self.status_bar()
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let toolbar = row![
            tb_button("← 戻る", Message::SwitchMode(self.prev_mode)),
            Space::new().width(Length::Fill),
            text("設定").size(16),
        ]
        .spacing(8)
        .padding(8)
        .align_y(iced::Alignment::Center);

        let theme_row = row![
            text("カラーテーマ").width(Length::Fixed(160.0)),
            pick_list(Theme::ALL, Some(self.theme.clone()), Message::ThemeSelected)
                .width(Length::Fixed(260.0)),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let font_row = row![
            text("フォント").width(Length::Fixed(160.0)),
            pick_list(FONT_NAMES, Some(self.font_name), Message::FontSelected)
                .width(Length::Fixed(260.0)),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let support = row![
            button(text("☕ 開発を支援する (Ko-fi)").size(14))
                .on_press(Message::OpenUrl(DONATE_URL))
                .padding([6, 14])
                .style(button::primary),
            button(text("GitHub リポジトリ").size(14))
                .on_press(Message::OpenUrl(REPO_URL))
                .padding([6, 14])
                .style(button::secondary),
        ]
        .spacing(10);

        let about = column![
            text("このアプリについて").size(20),
            text("Opsi-Lite v0.1.0").size(14),
            text("Obsidian互換・超軽量Markdownエディタ (Rust + iced)").size(14),
            text("ライセンス: MIT (無料配布・改変・再配布可)").size(14),
            text("無料です。役に立ったら下のボタンから支援いただけると励みになります。").size(13),
            support,
            container(scrollable(
                text(LICENSE_TEXT).font(Font::MONOSPACE).size(12)
            ))
            .height(Length::Fixed(200.0))
            .width(Length::Fill)
            .padding(8)
            .style(container::bordered_box),
        ]
        .spacing(8);

        let body = scrollable(
            column![
                text("表示設定").size(20),
                theme_row,
                font_row,
                Space::new().height(Length::Fixed(16.0)),
                about,
            ]
            .spacing(14)
            .padding(20)
            .max_width(720),
        )
        .height(Length::Fill);

        column![toolbar, body, self.status_bar()]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn edit_targets(&self) -> (Option<usize>, Option<(usize, usize)>) {
        match self.edit {
            Edit::Line(i) => (Some(i), None),
            Edit::Cell { line, col } => (None, Some((line, col))),
            Edit::None => (None, None),
        }
    }

    fn suggestion_panel(&self) -> Element<'_, Message> {
        let mut col = Column::new().spacing(1).padding(4);
        col = col.push(text("WikiLink 候補 (クリックで挿入 / Escで閉じる)").size(11));
        for s in self.suggestions.iter().take(10) {
            col = col.push(
                button(text(s.clone()).size(13))
                    .width(Length::Fill)
                    .padding([2, 6])
                    .on_press(Message::SuggestionChosen(s.clone()))
                    .style(button::secondary),
            );
        }
        container(col)
            .width(Length::Fill)
            .max_height(200)
            .style(container::dark)
            .into()
    }

    // ================= 編集ロジック =================

    fn cursor_lc(&self) -> (usize, usize) {
        let p = self.content.cursor().position;
        (p.line, p.column)
    }

    /// 本文を丸ごと差し替える(プレビュー側のインライン編集用)。
    fn set_content(&mut self, s: &str) {
        self.content = text_editor::Content::with_text(s);
        self.dirty = true;
        self.revision += 1;
    }

    fn replace_line(&mut self, idx: usize, new: &str) {
        let mut lines: Vec<String> = self.text_cache.split('\n').map(String::from).collect();
        if idx < lines.len() {
            lines[idx] = new.to_string();
        } else {
            while lines.len() <= idx {
                lines.push(String::new());
            }
            lines[idx] = new.to_string();
        }
        self.set_content(&lines.join("\n"));
    }

    fn insert_line_after(&mut self, idx: usize) {
        let mut lines: Vec<String> = self.text_cache.split('\n').map(String::from).collect();
        let at = (idx + 1).min(lines.len());
        lines.insert(at, String::new());
        self.set_content(&lines.join("\n"));
    }

    fn refresh_suggestions(&mut self) {
        let (line, col) = self.cursor_lc();
        let text = self.content.text();
        let line_text = text.split('\n').nth(line).unwrap_or("");
        let byte_col = char_to_byte(line_text, col);
        let before = &line_text[..byte_col.min(line_text.len())];
        self.refresh_suggestions_from(before);
    }

    fn refresh_suggestions_from(&mut self, before_cursor: &str) {
        match wikilink::active_completion_query(before_cursor) {
            Some(q) => {
                self.suggestions = self.index.complete(&q);
                self.suggestion_query = Some(q);
            }
            None => {
                self.suggestions.clear();
                self.suggestion_query = None;
            }
        }
    }

    fn choose_suggestion(&mut self, name: String) -> Task<Message> {
        match self.edit {
            Edit::Line(idx) => {
                // 行編集中: バッファ内の最後の `[[クエリ` を `[[name]]` へ。
                if let Some(open) = self.edit_buffer.rfind("[[") {
                    let nb = format!("{}{}]]", &self.edit_buffer[..open + 2], name);
                    self.edit_buffer = nb.clone();
                    self.replace_line(idx, &nb);
                }
                self.suggestions.clear();
                self.suggestion_query = None;
                operation::focus(preview::ACTIVE_INPUT_ID)
            }
            _ => {
                // Source エディタ: クエリ分 Backspace して挿入。
                self.apply_suggestion(name);
                Task::none()
            }
        }
    }

    fn apply_suggestion(&mut self, name: String) {
        if let Some(q) = self.suggestion_query.take() {
            use text_editor::{Action, Edit as TEdit};
            for _ in 0..q.chars().count() {
                self.content.perform(Action::Edit(TEdit::Backspace));
            }
            let insert = format!("{name}]]");
            self.content
                .perform(Action::Edit(TEdit::Paste(Arc::new(insert))));
            self.dirty = true;
            self.revision += 1;
        }
        self.suggestions.clear();
    }

    fn toggle_task(&mut self, line_idx: usize) {
        let text = self.content.text();
        if let Some(new_text) = tasks::toggle_in_text(&text, line_idx) {
            self.set_text_preserving_cursor(&new_text);
            self.dirty = true;
            self.revision += 1;
            self.status = "タスクをトグルしました".into();
        }
    }

    fn write_table_cell(&mut self, line: usize, col: usize, value: &str) {
        let mut lines: Vec<String> = self.text_cache.split('\n').map(String::from).collect();
        if let Some(l) = lines.get(line) {
            if let Some(new_line) = table::set_cell(l, col, value) {
                lines[line] = new_line;
                self.set_content(&lines.join("\n"));
            }
        }
    }

    fn set_text_preserving_cursor(&mut self, s: &str) {
        use text_editor::{Action, Motion};
        let (line, col) = self.cursor_lc();
        self.content = text_editor::Content::with_text(s);
        self.content.perform(Action::Move(Motion::DocumentStart));
        for _ in 0..line {
            self.content.perform(Action::Move(Motion::Down));
        }
        self.content.perform(Action::Move(Motion::Home));
        for _ in 0..col {
            self.content.perform(Action::Move(Motion::Right));
        }
    }

    fn follow_link_under_cursor(&mut self) -> Task<Message> {
        let (line, col) = self.cursor_lc();
        let text = self.content.text();
        let line_text = text.split('\n').nth(line).unwrap_or("").to_string();
        if let Some(link) = wikilink::link_at_column(&line_text, col) {
            let target = link.inner.split('|').next().unwrap_or("").trim().to_string();
            return self.follow_link(target);
        }
        self.status = "カーソル位置にWikiLinkがありません".into();
        Task::none()
    }

    fn follow_link(&mut self, name: String) -> Task<Message> {
        match self.index.resolve(&name, self.path.as_deref()) {
            Some(p) => self.open_path(p),
            None => match self.index.new_file_path(&name, self.path.as_deref()) {
                Some(new_path) => {
                    self.status = format!("'{name}' は存在しません");
                    let np = new_path.clone();
                    Task::perform(
                        async move {
                            let res = rfd::AsyncMessageDialog::new()
                                .set_title("新規ファイル作成")
                                .set_description(format!(
                                    "ノート '{}' は存在しません。\n新規作成して開きますか？",
                                    np.file_name().and_then(|n| n.to_str()).unwrap_or("")
                                ))
                                .set_buttons(rfd::MessageButtons::YesNo)
                                .show()
                                .await;
                            (new_path, matches!(res, rfd::MessageDialogResult::Yes))
                        },
                        |(p, yes)| {
                            if yes {
                                Message::CreateAndOpen(p)
                            } else {
                                Message::Noop
                            }
                        },
                    )
                }
                None => {
                    self.status = "リンク先を決定できません(ルート未設定)".into();
                    Task::none()
                }
            },
        }
    }

    fn open_path(&mut self, p: PathBuf) -> Task<Message> {
        if let Some(cur) = &self.path {
            if cur != &p {
                self.history.push(cur.clone());
            }
        }
        let path = p.clone();
        Task::perform(load(p), move |r| Message::Loaded(path.clone(), r))
    }

    fn save(&mut self) -> Task<Message> {
        match &self.path {
            Some(p) => {
                let p = p.clone();
                let body = self.content.text();
                Task::perform(write_file(p, body), Message::Saved)
            }
            None => {
                let body = self.content.text();
                Task::perform(
                    async move {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter("Markdown", &["md", "markdown"])
                            .set_file_name("untitled.md")
                            .save_file()
                            .await;
                        match handle {
                            Some(h) => write_file_inner(h.path().to_path_buf(), body).await,
                            None => Err("保存がキャンセルされました".into()),
                        }
                    },
                    Message::Saved,
                )
            }
        }
    }

    fn save_silent(&mut self) -> Task<Message> {
        if let Some(p) = &self.path {
            let p = p.clone();
            let body = self.content.text();
            Task::perform(write_file(p, body), Message::Saved)
        } else {
            Task::none()
        }
    }
}

// ---- 補助 ----

fn tb_button(label: &str, msg: Message) -> iced::widget::Button<'_, Message> {
    button(text(label.to_string()).size(13))
        .on_press(msg)
        .padding([4, 10])
}

fn theme_from_name(name: &str) -> Theme {
    Theme::ALL
        .iter()
        .find(|t| t.to_string() == name)
        .cloned()
        .unwrap_or(Theme::TokyoNight)
}

fn font_from_name(name: &str) -> (Font, &'static str) {
    for &n in FONT_NAMES {
        if n == name {
            return (Font::with_name(n), n);
        }
    }
    (Font::with_name(FONT_NAMES[0]), FONT_NAMES[0])
}

// ---- 非同期 I/O ----

fn scan_task(root: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(index::scan(root));
            });
            rx.await.unwrap_or_default()
        },
        Message::Indexed,
    )
}

async fn load(p: PathBuf) -> Result<String, String> {
    let (tx, rx) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(std::fs::read_to_string(&p).map_err(|e| e.to_string()));
    });
    rx.await.unwrap_or_else(|_| Err("読込スレッド失敗".into()))
}

async fn write_file(p: PathBuf, body: String) -> Result<PathBuf, String> {
    write_file_inner(p, body).await
}

async fn write_file_inner(p: PathBuf, body: String) -> Result<PathBuf, String> {
    let (tx, rx) = iced::futures::channel::oneshot::channel();
    let p2 = p.clone();
    std::thread::spawn(move || {
        let r = std::fs::write(&p2, body).map(|_| p2).map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    rx.await.unwrap_or_else(|_| Err("書込スレッド失敗".into()))
}

fn char_to_byte(s: &str, char_col: usize) -> usize {
    s.char_indices().nth(char_col).map(|(b, _)| b).unwrap_or(s.len())
}
