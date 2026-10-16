# ディレクトリ構成

```
crates/hyperdeck-pool/    複数台の TCP、再接続、notify、状態キャッシュ
crates/hyperdeck-plugin/  Stream Deck プラグイン本体とアクション
pi/                       Property Inspector (React)
plugin/dev.mikanseilaboratory.bmd.hyperdeck.sdPlugin/
  manifest.json
  en.json / ja.json
  images/
  layouts/
  propertyinspector/      Vite のビルド出力
  bin/plugin              macOS ユニバーサルバイナリ
  bin/plugin.exe          Windows 実行ファイル
publish.sh / publish.ps1
```
