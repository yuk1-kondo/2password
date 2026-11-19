# 2Password ブラウザ拡張

MV3拡張 + native messaging。ホスト・アプリ側の中継と番号選択式chooserは実装済み。実ブラウザE2Eは未検証。

## 設計
- 拡張はフィールド検出とUIのみ。全てのVault操作は `sendNativeMessage` で
  デスクトップアプリ側のネイティブホストに委譲（KeePassXC方式）
- メッセージ: `findLogins {url}` / `getPassword {uuid,url}` / 将来 `unlock`
- セキュリティ方針: ドメイン厳密一致のみ自動フィル、複数一致時はchooser表示

## macOS / Chromeでの検証

1. `chrome://extensions` のデベロッパーモードを有効にし、「パッケージ化されていない拡張機能を読み込む」で `apps/extension` を指定する。
2. 表示された拡張IDをコピーし、リポジトリ直下で `./scripts/install-native-host.sh <拡張ID>` を実行する。`cargo` がPATHに必要。
3. 2Passwordアプリを開き、ユーザー自身でアンロックする。
4. テスト用ログインを登録したドメインのフォームを開き、パスワード欄横の🔑から入力を確認する。送信は不要。
5. 同一ドメインにテスト用ログインを2件用意し、番号選択で各候補を入力できることを確認する。
6. アプリをロックして取得できないこと、不一致ドメインでは候補が出ないことを確認する。

Chromeの `allowed_origins` はワイルドカード非対応のため、実際の拡張IDを登録する。別ブラウザでIDが異なる場合は、そのIDで登録し直す。
Firefox / Safariでの動作は未検証。

## 自動検証

`node --test apps/extension/tests/registration.test.cjs`

拡張の参照ファイルと、隔離したHOMEでの登録・不正ID拒否を検証する。実Vaultやブラウザ設定は変更しない。
