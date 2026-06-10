// Opsi-Lite: Obsidian互換・超軽量Markdownエディタ
//
// 仕様書 3.1 非機能要件:
//   - 起動からウィンドウ描画＋内容レンダリングまで 200ms 以内
//   - 定常メモリ消費 30MB 以下
// を達成するため、ネイティブGUI(iced) + コンパイル言語(Rust) を採用する。
//
// エントリポイントはCLI引数(argv[1])を解釈し、Iced アプリケーションを起動する。

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod config;
mod highlight;
mod index;
mod preview;
mod table;
mod tasks;
mod wikilink;

use app::App;

fn main() -> iced::Result {
    // 仕様 4.1 / 3.2: コマンドライン引数からファイル(またはフォルダ)パスを受け取る。
    let initial_arg = std::env::args().nth(1).map(std::path::PathBuf::from);

    // ウィンドウアイコン(タスクバー/タイトルバー)。raw RGBA を埋め込み。
    let icon = iced::window::icon::from_rgba(
        include_bytes!("../assets/icon.rgba").to_vec(),
        256,
        256,
    )
    .ok();
    let window = iced::window::Settings {
        size: iced::Size::new(1100.0, 720.0),
        icon,
        ..Default::default()
    };

    // iced 0.14: boot(状態初期化), update, view を渡し、ビルダーで設定。
    let mut app = iced::application(
        move || App::new(initial_arg.clone()),
        App::update,
        App::view,
    )
    .title(App::title)
    .subscription(App::subscription)
    .theme(App::theme)
    .antialiasing(true)
    .window(window);

    // UI(ツールバー・プレビュー・ステータス)で日本語(CJK)を表示するため、
    // OSに存在する日本語対応フォントを既定フォントに設定する。
    // (エディタ本文は等幅フォントを別途指定している。)
    #[cfg(target_os = "windows")]
    {
        app = app.default_font(iced::Font::with_name("Yu Gothic UI"));
    }
    #[cfg(target_os = "macos")]
    {
        app = app.default_font(iced::Font::with_name("Hiragino Sans"));
    }

    app.run()
}
