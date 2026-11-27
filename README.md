# 2Password

1Passwordのセキュリティ設計（2SKD・階層鍵・ゼロ知識）をベースにした、個人利用のパスワードマネージャー。
A personal password manager modeled on 1Password's security design (2SKD, hierarchical keys, zero-knowledge).

> [!CAUTION]
> **個人の未監査プロジェクトです。自己責任でご利用ください。**
> 第三者によるセキュリティ監査、SOC2、バグバウンティ等は未実施であり、1Passwordと同等の保証には達していません。重要な資格情報の唯一の保管場所として使わないでください。
>
> **This is an unaudited personal project — use at your own risk.** No third-party security audit, SOC2, or bug bounty exists, and it does NOT provide 1Password-equivalent assurance. Do not use it as the sole store for important credentials.

## セキュリティモデル概要

- **2SKD**: マスターパスワード（Argon2id）と端末ごとのSecret KeyからAUKを導出。どちらかだけでは復号不可
- **階層鍵**: AUKでkeysetをwrap、keysetがアイテム鍵をwrap、アイテム鍵が各フィールドを暗号化
- **ゼロ知識**: 秘密は全てクライアント側で暗号化。同期先（iCloud Drive等）には暗号文のみ保存
- **Secret Keyは端末のみ**: Keychain（macOS/iOS）・Keystore（Android）に保持し、クラウドへは一切アップロードしない
- 漏洩チェックはHIBPのk-anonymity方式（SHA-1先頭5文字のみ送信）

仕様の詳細・脅威モデルは [`docs/security-spec.md`](docs/security-spec.md) が唯一の基準。

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
apps/windows/   設計メモ + C APIのみ（Windowsアプリは未作成）
docs/           security-spec.md — 脅威モデルと暗号仕様（実装の唯一の基準）
```

## プラットフォーム状況

| プラットフォーム | 状態 |
|---|---|
| macOS | **実用レベル** — Touch ID / iCloud Drive同期 / クリップボード消去 / 自動ロック / CSV / TOTP / 漏洩チェック / ブラウザ拡張基盤 |
| CLI (`2pw`) | コア検証用として動作 |
| iOS | ビルド・画面表示を確認済み（実機・Face ID未検証、配布ビルドなし） |
| Android | UI検証済み — Keystore暗号化SK / BiometricPrompt / 同機能群（実サイトでのAutofill未検証） |
| ブラウザ拡張 | 基盤のみ — native messaging経路とchooserは実装済み、実ブラウザE2E未検証 |
| Windows | **アプリ未作成** — C API (`tp_*`) と設計メモのみ。WinUI 3側の実装が必要 |

## ビルド

### 前提

- Rust（rustup経由推奨。Homebrewのrustはホスト専用でiOS/Android向けstdlibを持たない）
- macOS/iOS: Xcode + [xcodegen](https://github.com/yonsm/XcodeGen)
- Android: Android SDK + NDK 27.x、`rustup target add aarch64-linux-android x86_64-linux-android`
- Node.js（ブラウザ拡張のテスト用）

### コア（Rust）

```sh
cargo build          # 全クレート
cargo test           # twopassword-coreの32テスト
cargo build --release -p twopassword-cli   # 2pw CLI
```

### macOS / iOS

```sh
./scripts/build-apple.sh     # Rust→XCFramework→Swiftバインディング（macOS+iOS両対応）
cd apps/apple
xcodegen                                     # TwoPassword.xcodeprojを生成
xcodebuild -scheme TwoPassword-macOS build   # macOSアプリ
xcodebuild -scheme TwoPassword-iOS build     # iOS（シミュレータ向け）
```

### Android

```sh
./scripts/build-android.sh   # Rust→.so→Kotlinバインディング→APK (debug)
# 出力: apps/android/app/build/outputs/apk/debug/app-debug.apk
```

### ブラウザ拡張

Chrome `chrome://extensions` でデベロッパーモードを有効化し、`apps/extension` を読み込む。表示された拡張IDで：

```sh
./scripts/install-native-host.sh <拡張ID>
node --test apps/extension/tests/registration.test.cjs   # 登録ロジックのテスト
```

手順の詳細は [`apps/extension/README.md`](apps/extension/README.md) を参照。

### Windows

Mac上ではビルド不可。設計メモと作業手順は [`apps/windows/README.md`](apps/windows/README.md) を参照（Windows機での実装作業が必要）。

## 設計上の注意

- パスワード等の秘密は全てクライアント側で暗号化、クラウドは暗号文のみ
- Secret Keyは端末のみ存在（Keychain/Keystore）、iCloudにもアップロードしない
- 1Passwordとの完全な保証同等は未達（監査・SOC2・バグバウンティ等は未実施）

## License

MIT — Copyright (c) 2026 Yuki Kondo. See [LICENSE](LICENSE).
