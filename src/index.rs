// 仕様 5.1 非同期フォルダスキャン / 5.2 リンク解決アルゴリズム
//
// ファイルオープン時に親フォルダ(テンポラリ・ルート)を walkdir で走査し、
// 「ファイル名(stem) → 絶対パス群」のインデックスをメモリ上に構築する。
// このスキャンは iced::Task::perform 経由でワーカースレッド上で実行され、
// GUI描画スレッドをブロッキングしない (仕様 5.1)。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// テンポラリ・ルート配下の Markdown ファイル索引。
#[derive(Debug, Clone, Default)]
pub struct Index {
    /// 小文字化したファイル名(拡張子なし stem) → 絶対パス群
    by_stem: HashMap<String, Vec<PathBuf>>,
    /// 補完候補表示用に、元の表記の stem を昇順で保持
    stems: Vec<String>,
    /// 走査の起点(テンポラリ・ルート)
    root: Option<PathBuf>,
}

impl Index {
    /// 補完候補に使える全ノート名(stem)を返す。
    pub fn all_stems(&self) -> &[String] {
        &self.stems
    }

    #[allow(dead_code)]
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// 仕様 4.2 自動補完: 前方一致・部分一致のインクリメンタルサーチ。
    /// 前方一致を優先して並べ、その後に部分一致を続ける。
    pub fn complete(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return self.stems.iter().take(50).cloned().collect();
        }
        let mut prefix = Vec::new();
        let mut contains = Vec::new();
        for s in &self.stems {
            let sl = s.to_lowercase();
            if sl.starts_with(&q) {
                prefix.push(s.clone());
            } else if sl.contains(&q) {
                contains.push(s.clone());
            }
        }
        prefix.extend(contains);
        prefix.truncate(50);
        prefix
    }

    /// 仕様 5.2 リンク解決アルゴリズム。
    /// `current` は現在開いているファイルの絶対パス(なければ None)。
    /// 戻り値: 解決された既存ファイルのパス。存在しなければ None
    ///         (呼び出し側で「新規作成フラグ」を立てる — 仕様 5.2-4)。
    pub fn resolve(&self, name: &str, current: Option<&Path>) -> Option<PathBuf> {
        let stem = normalize_link_name(name);

        // 1. カレントファイルと同一ディレクトリを最優先。
        if let Some(cur) = current {
            if let Some(dir) = cur.parent() {
                let cand = dir.join(format!("{stem}.md"));
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }

        // 2. テンポラリ・ルート直下。
        if let Some(root) = &self.root {
            let cand = root.join(format!("{stem}.md"));
            if cand.is_file() {
                return Some(cand);
            }
        }

        // 3. サブディレクトリを含むインデックスから最初の一致を返す
        //    (walkdir は決定的順序のため「最初に見つかったもの」が安定する)。
        if let Some(paths) = self.by_stem.get(&stem.to_lowercase()) {
            // カレントと同一ディレクトリのものを念のため優先
            if let Some(cur_dir) = current.and_then(|c| c.parent()) {
                if let Some(p) = paths.iter().find(|p| p.parent() == Some(cur_dir)) {
                    return Some(p.clone());
                }
            }
            return paths.first().cloned();
        }

        // 4. 完全に存在しない。
        None
    }

    /// 解決できなかったリンクに対する新規作成先パスを決定する (仕様 4.2 / 5.2-4)。
    /// カレントファイルと同一ディレクトリ、なければテンポラリ・ルート直下に作る。
    pub fn new_file_path(&self, name: &str, current: Option<&Path>) -> Option<PathBuf> {
        let stem = normalize_link_name(name);
        if let Some(dir) = current.and_then(|c| c.parent()) {
            return Some(dir.join(format!("{stem}.md")));
        }
        self.root.as_ref().map(|r| r.join(format!("{stem}.md")))
    }
}

/// `[[名前]]` / `[[名前|表示名]]` / `[[名前#見出し]]` から実ファイル名部分を取り出す。
pub fn normalize_link_name(name: &str) -> String {
    let n = name.trim();
    // 表示名(|)・見出し(#)・ブロック参照(^) を除去
    let n = n.split('|').next().unwrap_or(n);
    let n = n.split('#').next().unwrap_or(n);
    let n = n.split('^').next().unwrap_or(n);
    n.trim().trim_end_matches(".md").to_string()
}

/// 仕様 5.1: 指定ルート配下を walkdir で走査し Index を構築する。
/// 重い I/O は呼び出し側の Task::perform 経由でワーカースレッドに載せる前提。
pub fn scan(root: PathBuf) -> Index {
    let mut by_stem: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut stem_set: Vec<String> = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        let is_md = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
            .unwrap_or(false);
        if !is_md {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            by_stem
                .entry(stem.to_lowercase())
                .or_default()
                .push(path.to_path_buf());
            stem_set.push(stem.to_string());
        }
    }

    stem_set.sort_unstable();
    stem_set.dedup();

    Index {
        by_stem,
        stems: stem_set,
        root: Some(root),
    }
}
