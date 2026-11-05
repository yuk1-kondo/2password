# 2Password ブラウザ拡張（スキャフォールド）

状態: **骨格のみ**。MV3拡張 + native messaging 方針（仕様§8.3: 秘密は拡張プロセスに持たない）。

## 設計
- 拡張はフィールド検出とUIのみ。全てのVault操作は `sendNativeMessage` で
  デスクトップアプリ側のネイティブホストに委譲（KeePassXC方式）
- メッセージ: `findLogins {url}` / `getPassword {uuid,url}` / 将来 `unlock`
- セキュリティ方針: ドメイン厳密一致のみ自動フィル、複数一致時はchooser表示

## 残タスク
1. ネイティブホスト実装（crates/nmh: stdin/stdout 長さプレフィックスプロトコル）
   - アプリ内のUnlockedVault参照 or 専用ロック解除フローが必要
   - マニフェスト登録: `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/dev.twopassword.native.json`
2. chooser UI（複数一致時）
3. Safari版（Xcodeでsafari web extension変換）
