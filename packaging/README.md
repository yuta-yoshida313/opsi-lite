# パッケージ配布マニフェスト

`winget` / `Scoop` で配布するためのマニフェストです。リリース（タグ `vX.Y.Z`）ごとに
URL と SHA256 を更新してください。

## SHA256 の取得
```powershell
(Get-FileHash .\dist\opsi-lite-vX.Y.Z-windows-x64.exe -Algorithm SHA256).Hash
```

## winget（Microsoft公式リポジトリへ申請）
1. `wingetcreate` を使うのが簡単です:
   ```powershell
   winget install wingetcreate
   wingetcreate update YoshidaSoftware.OpsiLite --version 0.1.0 `
     --urls https://github.com/yuta-yoshida313/opsi-lite/releases/download/v0.1.0/opsi-lite-v0.1.0-windows-x64.exe `
     --submit
   ```
   `--submit` で `microsoft/winget-pkgs` へ自動PRされます（GitHubアカウント連携が必要）。
2. 手動の場合は `winget/` 配下のYAMLを `microsoft/winget-pkgs` の
   `manifests/y/YoshidaSoftware/OpsiLite/0.1.0/` に置いてPR。
3. マージ後: `winget install YoshidaSoftware.OpsiLite`

## Scoop
- 自分の Scoop バケット（例: `yuta-yoshida313/scoop-bucket` リポジトリ）に
  `scoop/opsi-lite.json` を置くと、利用者は次で入れられます:
  ```powershell
  scoop bucket add yoshida https://github.com/yuta-yoshida313/scoop-bucket
  scoop install opsi-lite
  ```
- 公式 `extras` バケットへ入れたい場合は scoop-extras へPR。

> いずれも **未署名exe** のため、初回はSmartScreen警告が出る場合があります（README参照）。
> コード署名を導入すると警告は軽減/解消します。
