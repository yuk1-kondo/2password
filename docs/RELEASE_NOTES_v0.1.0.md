# 2Password v0.1.0 — 初回公開リリース

1Passwordのセキュリティ設計（2SKD・階層鍵・ゼロ知識）をベースにした、個人利用のパスワードマネージャーの初回公開版です。

> [!CAUTION]
> **個人の未監査プロジェクトです。自己責任でご利用ください。**
> 第三者セキュリティ監査・SOC2・バグバウンティ等は未実施であり、1Passwordと同等の保証には達していません。重要な資格情報の唯一の保管場所として使わないでください。
>
> **This is an unaudited personal project — use at your own risk.** It does NOT provide 1Password-equivalent assurance.

## 機能

- **暗号コア**: 2SKD（マスターパスワード Argon2id + 端末別Secret Key → AUK）による階層鍵。XChaCha20-Poly1305 AEAD
- **Vault**: アイテムの作成/編集/削除（tombstone）、keyset再wrapによるマスターパスワード変更
- **パスワード生成**: ランダムパスワード / パスフレーズ（7776語ワードリスト）/ PIN
- **TOTP**: RFC 6238準拠のワンタイムパスコード
- **漏洩チェック**: HIBP k-anonymity（SHA-1先頭5文字のみ送信）
- **CSV インポート/エクスポート**
- **エマージェンシーキット + QR**: Secret Keyの紙媒体バックアップ
- **macOS**: Touch ID、iCloud Drive同期（conflict LWW dedupe）、クリップボード90秒消去、自動ロック
- **Android**: Keystore暗号化Secret Key、BiometricPromptでAUK保護、AutofillService
- **ブラウザ拡張（基盤）**: MV3 + native messaging（`2pw-nmh`）、ドメイン厳密一致フィル、複数一致chooser
- **CLI `2pw`**: init / add / get / list / rm / gen

## プラットフォーム状況

| プラットフォーム | 状態 |
|---|---|
| macOS | 実用レベル — 上記全機能が動作。Releaseビルド起動確認済み |
| CLI (`2pw`) | 動作（macOS用releaseバイナリ添付） |
| iOS | ビルド・画面表示確認済み。実機・Face ID未検証、配布ビルドなし |
| Android | debug APK ビルド・UI検証済み。実サイトでのAutofill未検証 |
| ブラウザ拡張 | 基盤のみ。実ブラウザE2Eは未検証（手順は `apps/extension/README.md`） |
| Windows | アプリ未作成。C API（`tp_*` 18関数）と設計メモのみ |

## 添付アーティファクト

- `2Password-macOS.zip` — macOSアプリ（arm64、ad-hoc署名 "Sign to Run Locally"）
- `2pw` — CLIバイナリ（macOS arm64, releaseビルド）
- `2pw-nmh` — ネイティブメッセージングホスト（ブラウザ拡張用）
- `SHA256SUMS.txt` — 各アーティファクトのSHA-256チェックサム

※ 署名はad-hocのため、初回起動時は右クリック→「開く」から開いてください。Gatekeeperの警告が出ます。
※ Android debug APK は開発者検証用で、リリース成果物には含めていません。

## 既知の制限

- 同期はiCloud Driveのみ（macOS）。他のマルチデバイス経路は未実装
- iOS AutoFill拡張は未実装（App Groupエンタイトルメント＝Apple開発者アカウントが必要）
- パスキー未対応
- QR新端末登録はキット表示側のみ（スキャン側未実装）
- ブラウザ拡張は登録手順が手動（拡張IDを`install-native-host.sh`で登録）
- 第三者監査なし。セキュリティ仕様は `docs/security-spec.md` を参照

## セキュリティ

脆弱性らしきものを見つけた場合は公開issueではなく、リポジトリオーナーへ直接ご連絡ください。
