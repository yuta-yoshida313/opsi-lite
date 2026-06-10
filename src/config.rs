// 設定の永続化(カラーテーマ・フォント)。
// Windows: %APPDATA%\Opsi-Lite\config.txt / その他: ~/.config/opsi-lite/config.txt
// 依存を増やさないため単純な key=value 形式で保存する。

use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub theme: Option<String>,
    pub font: Option<String>,
}

fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);

    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));

    base.map(|b| b.join("Opsi-Lite").join("config.txt"))
}

pub fn load() -> Config {
    let mut cfg = Config::default();
    if let Some(path) = config_path() {
        if let Ok(s) = std::fs::read_to_string(&path) {
            for line in s.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    let v = v.trim().to_string();
                    match k.trim() {
                        "theme" => cfg.theme = Some(v),
                        "font" => cfg.font = Some(v),
                        _ => {}
                    }
                }
            }
        }
    }
    cfg
}

pub fn save(theme: &str, font: &str) {
    if let Some(path) = config_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, format!("theme={theme}\nfont={font}\n"));
    }
}
