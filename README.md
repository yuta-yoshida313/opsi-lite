# Opsi-Lite

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![ko-fi](https://img.shields.io/badge/Support-Ko--fi-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/yoshidasoftware)

Obsidian互換・超軽量Markdownエディタ（Rust + iced ネイティブGUI）。

「保管庫（Vault）」の概念に縛られず、ローカルの任意フォルダにある Markdown を即座に開き、
WikiLink・タスクトグルなど Obsidian の主要記法を快適に編集・閲覧するためのスタンドアロンエディタです。

## 特長（仕様対応）

| 区分 | 内容 |
|------|------|
| 技術スタック | Rust / iced 0.14（ネイティブGUI・tiny-skiaソフトウェアレンダラ）/ 行単位インクリメンタルハイライタ（Tree-sitterへ差替可） |
| 画面構成 | 既定は**プレビュー主画面**。行をクリックするとその場で編集（Obsidian Live Preview方式）。右上「</> コード」で生Markdown編集、「⚙ 設定」で設定画面 |
| インライン編集 | プレビュー上で段落・見出し・引用・箇条書き・タスク・テーブルをすべて編集可能。`Enter`で次行追加 |
| 日本語入力 | iced 0.14 の IME（Input Method）対応により、日本語・中国語等のIME入力に対応 |
| 設定 | カラーテーマ（iced全テーマ）とフォントを設定画面で変更し、`config.txt`へ永続化。About/ライセンスも表示 |
| 起動 | CLI引数 `opsi-lite <file>` またはフォルダ指定。親フォルダを自動的にテンポラリ・ルートとして認識（仕様 4.1） |
| WikiLink | `[[` 入力で補完候補をポップアップ（前方一致＋部分一致）。F12 でジャンプ。未存在なら新規作成を確認（仕様 4.2） |
| タスク | `Ctrl+Enter` で `- [ ]`⇔`- [x]` トグル。プレビューのチェックボックスクリックでも本文を書き換え（仕様 4.3） |
| テーブル | プレビューでGFMテーブルをグリッド表示。セルをクリックでその場編集→本文へ反映。`Enter`で次セルへ。「表を挿入」でテンプレート挿入 |
| 索引 | 起動時に専用スレッドで `walkdir` 走査し、GUIをブロックせずに索引構築（仕様 5.1） |
| リンク解決 | 同一ディレクトリ → ルート直下 → サブディレクトリ再帰の順で探索（仕様 5.2） |
| アイコン | モダンな角丸グラデーション＋Markdownマーク。ウィンドウ/タスクバー/実行ファイルに適用 |
| ライセンス | MIT（配布可）。`LICENSE` 同梱・設定画面で全文表示 |

## キーボードショートカット

| キー | 動作 |
|------|------|
| `Ctrl/Cmd + Enter` | カーソル行のタスクをトグル |
| `F12` | カーソル直下の WikiLink へジャンプ |
| `Ctrl/Cmd + S` | 保存 |
| `Alt + ←` | 履歴を戻る |
| `Ctrl/Cmd + E` | プレビュー ⇔ コード(Source) 編集の切替 |
| `Esc` | 補完候補・テーブルセル編集を閉じる |
| `[[` | WikiLink 補完候補を表示（クリックで挿入） |

テーブル編集: プレビュー上のセルをクリック→`text_input`化してその場編集、`Enter`で同一行の次のセルへ移動。ツールバーの「表を挿入」で3列テンプレートをカーソル位置に挿入。

## ビルド

```powershell
# 開発ビルド
cargo run -- path\to\note.md

# リリースビルド（起動速度・メモリ最適化）
cargo build --release
.\target\release\opsi-lite.exe path\to\note.md
```

## OS統合（.md 関連付け） — 仕様 3.1

### Windows
1. 任意の `.md` ファイルを右クリック →「プログラムから開く」→「別のプログラムを選択」
2. `target\release\opsi-lite.exe` を指定し「常にこのアプリを使う」にチェック

または管理者 PowerShell で:
```powershell
$exe = (Resolve-Path .\target\release\opsi-lite.exe).Path
cmd /c "ftype OpsiLite.md=`"$exe`" `"%1`""
cmd /c "assoc .md=OpsiLite.md"
```

### macOS
`opsi-lite` を `.app` バンドル化し、Finder で `.md` の「情報を見る」→「このアプリケーションで開く」から既定に設定。

### Linux
`.desktop` ファイルを作成し `xdg-mime default opsi-lite.desktop text/markdown` を実行。

## アーキテクチャ

```
src/
  main.rs       エントリ・CLI引数解釈
  app.rs        iced アプリ本体（状態・更新・ビュー・ショートカット）
  index.rs      非同期フォルダスキャン / リンク解決（仕様 5.1, 5.2）
  wikilink.rs   [[...]] の検出・補完トリガ解析（仕様 4.2）
  tasks.rs      タスク記法の検出・トグル（仕様 4.3）
  table.rs      GFMテーブルの解析・セル編集（書き戻し）
  highlight.rs  iced Highlighter 実装（行単位レキサ）
  preview.rs    プレビュー兼インラインエディタ描画（行/リンク/タスク/テーブル）
  config.rs     設定（テーマ・フォント）の永続化
assets/         アイコン（icon.png/.ico/.rgba）と生成スクリプト
build.rs        Windows実行ファイルへのアイコン埋め込み
LICENSE         MIT ライセンス
```

## サポート / 寄付

Opsi-Lite は **無料** で配布しています（MITライセンス）。
役に立ったら、開発の継続支援としてご寄付いただけると励みになります 🙏

- ☕ Ko-fi: https://ko-fi.com/yoshidasoftware

アプリ内では「⚙ 設定 → このアプリについて」からも支援ボタンを開けます。

## インストール

GitHub Releases から `opsi-lite.exe` をダウンロードして実行してください。
未署名のため初回起動時に Windows SmartScreen の警告が出る場合があります。
その場合は「詳細情報」→「実行」で起動できます。

```powershell
# ソースからビルドする場合
git clone https://github.com/yuta-yoshida313/opsi-lite.git
cd opsi-lite
cargo build --release
.\target\release\opsi-lite.exe path\to\note.md
```

### Tree-sitter について
仕様 2 では Tree-sitter を選定。現状はビルドの確実性と起動速度を優先し、
増分・行単位の高速レキサをハイライトバックエンドに採用しています。
`--features treesitter` で Tree-sitter バックエンドへ差し替えるための拡張ポイントを用意しています。
