# 技術スタック

変更する場合は承認を得ること。

- Rust 1.99.1 (edition 2024)
- `streamdeck-plugin` 0.1 (`macros`, `typegen`)
- `hyperdeck-tokio`（git rev `d01e592620673a2980a0167c5a1b05b439985451`）
- `tokio` 1
- Property Inspector: React 18、Vite 6、TypeScript 5、`@mikanseilaboratory/streamdeck-pi-client` 0.1.0
- 設定型の共有: `ts-rs` 10.1
- 対象: Windows x64、macOS arm64、macOS x64（配布時は `lipo` でユニバーサル `bin/plugin`）
- Stream Deck SDK 2
- 制御面: HyperDeck Ethernet Protocol、TCP 9993
