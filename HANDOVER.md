# 2Password — Codex引継書

> [!NOTE]
> **これは開発者向けの内部メモです。** 公開利用者向けの情報は [README.md](README.md) を参照してください。
> 環境固有のパス・デバイス名・作業メモを含みます（情報保全のため原文のまま残しています）。

1Password型2SKD設計の個人用パスワードマネージャー。Rustコア+各OSネイティブUI。

## 環境前提

- macOS (Darwin)。Rustは `rustup` 経由: `source ~/.cargo/env` または PATHに `~/.cargo/bin`
- Android SDK: `~/Library/Android/sdk`、NDK: `ndk/27.2.12479018`（`.cargo/config.toml`にリンカ設定済み）
- エミュレータAVD: `pwtest`（Pixel7相当, API34）— シリアルは `emulator-5554` 等で変動
- Xcode + xcodegen (project.yml → TwoPassword.xcodeproj を生成)
- テストVault: `~/.2password/device.json` にCLIのSK保存（CLI用）

## ビルド＆テスト

```sh
source ~/.cargo/env
cargo test -p twopassword-core          # 32テスト
./scripts/build-apple.sh                # Rust→XCFramework→Swiftバインディング
cd apps/apple && xcodegen && xcodebuild -scheme TwoPassword-macOS build
./scripts/build-android.sh              # Rust→.so→Kotlinバインディング→APK(エミュレータ向けx86_64含む)
./scripts/install-native-host.sh <拡張ID> # Chromeで拡張を読み込み、表示されたIDで登録
```

生成物（gitignore済み、再生成で復元）: `apps/android-bindings/`、`apps/apple/Bindings/`、`apps/apple/Frameworks/twopasswordFFI.xcframework/`、`apps/apple/TwoPassword.xcodeproj/`、各`build/`。

## アーキテクチャ要点

- **2SKD**: パスワード(Argon2id)→intermediate + Secret Key→AUK → keysetをwrap。`crates/core/src/crypto/twoskd.rs`
- **FFI**: `crates/ffi`はUniFFI(Swift/Kotlin)と`capi.rs`(plain C ABI, `tp_*`, JSON受け渡し, thread-localエラー)の二面
- **Vault**: `vault/`=profile.json+keyset.json+items/*.json（全て暗号化済み）。SKは端末のKeychain/Keystoreのみ、Vault外
- **ブラウザ拡張**: `crates/nmh`がNMプロトコル(4byte長+JSON)→UDS `~/Library/Application Support/2password/app.sock`→`AppBridge`（アンロック時のみ応答）
- **ドメイン照合**: `crates/core/src/domain.rs` eTLD+1（自前実装、publicsuffixクレートはAPI摩擦で却下した経緯あり）
- **Android SK保護**: `KeystoreHelper`でAES-GCM暗号化→`device.key`。`unlockWithBiometric`はBiometricPrompt+CryptoObjectでAUKを30秒ユーザー認証ゲート
- **iCloud同期(macOSのみ)**: `~/Library/Mobile Documents/com~apple~CloudDocs/2password/vault`。`.icloud`プレースホルダ再DL対応+conflict LWW dedupe(`dedupe_items`)
- **SK復元**: 両OSアンロック画面に「Secret Keyを手入力して復元」—成功後のみ永続化（typo汚染防止）

## 実装済み機能

暗号コア/生成器/TOTP/HIBP(k-anonymity)/CSVインポート・エクスポート/パスワード変更(キーセット再wrap)/エマージェンシーキット+QR/自動ロック/クリップボード90秒消去/Touch ID/Keystore+BiometricPrompt/AutofillService/ネイティブメッセージング/全OSクライアント骨格

## 残タスク（優先度順）

2026-11-14: Chrome登録の不正なワイルドカードと欠落アイコン参照を修正。隔離HOMEでの登録テスト2件成功。実環境のホスト登録は未実施。手順は `apps/extension/README.md`。実ブラウザE2Eは引き続き未完了。

1. **ブラウザ拡張E2E** — `install-native-host.sh`実行済みか要確認→Chromeの`chrome://extensions`で`apps/extension`を読み込み→github.com等でログインフォームフィル確認。アプリがアンロック状態であること（ユーザー操作必須）。chooser UIの複数一致動作も要検証
2. **iOS AutoFill拡張** — App Groupエンタイトルメント必須（Apple開発者アカウント）。Extension targetを`apps/apple`に追加、shared containerでVault参照
3. **Windows実機ビルド** — `apps/windows/README.md`参照。`tp_*` C ABIはコンパイル済み。WinUI3+csbindgenでC#側を実装、Windows Hello/DPAPIでAUK保護
4. **QRスキャン側** — `EmergencyKitView`のQR表示は済。iOS側にAVCaptureSessionでスキャン→SK復元呼出し
5. **iOS実機検証** — シミュレータでは動くがFace ID/実機未確認
6. **Android Autofill実サイト検証** — サービス登録済み（`settings put secure autofill_service dev.twopassword/dev.twopassword.TwoPasswordAutofillService`）。実ログインフォームでのフィル未確認
7. **長期**: パスキー、セキュリティ監査、マルチデバイス同期の本実装

## 既知のハマりどころ

- `cargo`が見つからない → `rustup`パッケージはkeg-only。`source ~/.cargo/env`必須
- XCFramework: ios-simのarm64+x86_64を`lipo`で1つに統合してから`xcodebuild -create-xcframework`
- iOS: `homeDirectoryForCurrentUser`/`List(selection:)`非対応→`#if os(macOS)`分岐済み。iCloud Driveパス方式もmacOSのみ（iOSはubiquityコンテナ＝有料アカウント）
- Android cc-rs: APIレベル付きclangを`CC_<triple>`で指定（`.cargo/config.toml`に設定済み）
- adb自動化: `input text`はフォーカス必須、TAB移動不可、`keyevent 4`はIME表示中のみIME解除（それ以外はアプリ終了）。エミュレータのシリアル番号は再起動で変わる
- `open`は既起動アプリを再起動しない → `pkill -f "2Password.app"`してから`open`
- UniFFIバインディング再生成: `cargo run -p twopassword-ffi --example uniffi-bindgen -- generate ...`（build-android.sh内のコマンド参照）

## セキュリティ注意

- 全ての秘密はクライアント側暗号化、クラウドは暗号文のみ。HIBPはSHA-1先頭5文字のみ送信
- `apps/nmh`のソケットはiCloud外（Application Support）に配置済み。変えないこと
- 「1Passwordと完全同等の保証」は未達: 監査/SOC2/バグバウンティ等は未実施（個人利用想定）

## 参照

- `docs/security-spec.md` — 暗号仕様の唯一の基準
- `~/agent-config/lessons/2password-crossplatform.md` — ビルド系ハマりどころの記録
- `~/agent-config/plans/2password_multiplatform.plan.md` — 全体計画

## UIリデザイン (2026-11-14)

- Nuevo.Tokyoを参照した白黒・等幅ラベル・余白・和色グラデーションの共通UIを追加。
- 適用: macOS/iOSのセットアップ・ロック解除・一覧・詳細・モノグラム、および拡張ポップアップ。
- `docs/design-language.md` に方針・実装範囲・検証結果を記載。
- macOS/iOS Simulatorの署名なしビルド成功。Mac解除後にロック解除画面・空の保管庫・追加フォーム表示とキャンセルを確認。0件のためデータのある一覧・詳細は未確認。
- Android/Windowsは未適用。ブラウザE2Eの未完了状態は変わらない。
