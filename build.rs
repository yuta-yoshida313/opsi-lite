// Windows: 実行ファイルへアイコンを埋め込む(エクスプローラ/タスクバー用)。
// rc.exe (Windows SDK) が見つからない環境では埋め込みをスキップする。

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // アイコン埋め込みに失敗してもビルドは継続(警告のみ)。
            println!("cargo:warning=icon embedding skipped: {e}");
        }
    }
}
