# 2Password

1Passwordのセキュリティ設計をベースにした、個人利用のパスワードマネージャー。
対象OS: macOS / Windows / iOS / Android

## ドキュメント

- `docs/security-spec.md` — 脅威モデルと暗号仕様（実装の唯一の基準）

## 構成

```
crates/core/   twopassword-core — 全プラットフォーム共通のRustコア
               （暗号・Vault・同期・生成器・TOTP・漏洩チェック）
```

クライアントはコアをFFI（UniFFI等）経由で利用する薄いUI：
SwiftUI (macOS/iOS) / Jetpack Compose (Android) / WinUI 3 (Windows)

## 開発

```sh
cargo build
cargo test
```
