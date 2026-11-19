# 2Password

1Passwordのセキュリティ設計（2SKD・階層鍵・ゼロ知識）をベースにした、個人利用のパスワードマネージャー。
対象OS: macOS / Windows / iOS / Android

## 構成

```
crates/core/    twopassword-core — 全プラットフォーム共通のRustコア
                （暗号・Vault・同期・生成器・TOTP・漏洩チェック・ドメイン照合）
crates/ffi/     UniFFI (Swift/Kotlin) + plain C ABI (csbindgen/C#用)
crates/cli/     2pw — コア検証用CLI
crates/nmh/     ブラウザ拡張↔アプリのネイティブメッセージングホスト
apps/apple/     SwiftUI (macOS/iOS共有) + XCFramework
apps/android/   Jetpack Compose (Kotlin) + Kotlinバインディング
apps/extension/ MV3ブラウザ拡張（native messaging方式）
apps/windows/   WinUI 3用 設計メモ + C API
docs/           security-spec.md — 脅威モデルと暗号仕様（実装の唯一の基準）
```

## プラットフォーム状況

| OS | 状態 |
|---|---|
| macOS | 実用レベル — Touch ID/iCloud Drive同期/クリップボード消去/自動ロック/CSV/TOTP/漏洩チェック/ブラウザ拡張基盤 |
| iOS | ビルド・画面検証済み（ローカルVault、実機未確認） |
| Android | 実用レベルUI検証済み — Keystore暗号化SK/BiometricPrompt/同機能群 |
| Windows | C API準備済み（tp_*関数、csbindgen→C# P/Invoke。実機ビルドはWindows機で） |

## 開発

```sh
cargo build && cargo test    # コア（32テスト）
./scripts/build-apple.sh     # Rust→XCFramework→Swiftバインディング（macOS+iOS両対応）
./scripts/build-android.sh   # Rust→.so→Kotlinバインディング→APK
./scripts/install-native-host.sh <拡張ID> # chrome://extensionsに表示されたIDで登録
```

## 設計上の注意

- パスワード等の秘密は全てクライアント側で暗号化、クラウドは暗号文のみ
- Secret Keyは端末のみ存在（Keychain/Keystore）、iCloudにもアップロードしない
- 1Passwordとの完全な保証同等は未達（監査・SOC2・バグバウンティ等は未実施）
