# 2Password セキュリティ仕様書

ドラフト v0.1 — 脅威モデルと暗号設計。実装の唯一の基準となる文書。
設計は 1Password Security Design White Paper（公開仕様）をベースに、
自前サーバーを持たないファイル同期モデルへ適合させたもの。

---

## 1. 目的とスコープ

- macOS / Windows / iOS / Android 向けパスワードマネージャー（個人利用）
- 目標：1Passwordの暗号設計と同等以上の機密性・完全性を持つクライアント
- 同期は自前サーバーではなく暗号化Vaultファイルを外部ストレージ
  （iCloud Drive / Google Drive / OneDrive 等）に置く方式

## 2. 脅威モデル

### 2.1 保護対象

- Vault内の機密データ（パスワード、TOTPシード、ノート等）
- アカウントパスワード、Secret Key、全ての暗号鍵
- アイテムのメタデータも可能な限り暗号化する（§6）

### 2.2 想定する攻撃者

| 攻撃者 | 防御方針 |
|---|---|
| 同期ストレージ事業者 / クラウドからVaultファイルを盗んだ者 | E2E暗号化。ファイルにSecret Keyを含めないため、盗まれても総当たり不可（§4） |
| 端末を盗んだ者 | 端末にはSecret Keyがあるため残る防御はアカウントパスワード。Argon2idで強化し、生体認証/端末ロックが第一線 |
| 実行中プロセスのメモリを読むマルウェア | 常時暗号化、使用後即zeroize、mlockのベストエフォート。完全防御は不可能（§9.4） |
| 通信傍受者 | 同期はストレージプロバイダのTLSに依存。Vaultは既にE2E暗号化済みのため二重防御 |
| フィッシングサイト | オートフィルはドメイン厳密一致のみ（§8.3）。パスキーで将来強化 |
| サプライチェーン攻撃 | 依存ピン留め、cargo audit/deny、再現可能ビルド（§10） |

### 2.3 スコープ外（正直な限界）

- 完全に侵害された端末上での動作（キーロガー等）は暗号では防げない
- 第三者監査・SOC2等の保証レベルは1Passwordと同等ではない。設計同等＋継続検証が目標
- 物理的アクセス＋洗練された攻撃者（コールドブート、サイドチャネル）

## 3. 暗号プリミティブ（crypto suite v1）

アルゴリズムは `profile.json` の `crypto` フィールドでバージョン管理し、
将来的な移行を可能にする。

| 用途 | アルゴリズム | パラメータ |
|---|---|---|
| パスワード伸展（primary KDF） | **Argon2id** | m=64 MiB, t=3, p=4, salt=128bit乱数, out=256bit |
| パスワード伸展（互換モード） | PBKDF2-HMAC-SHA256 | 650,000回（1Password現行値と同一） |
| 鍵分離・合成 | HKDF-SHA256 | info に用途タグを必須付与 |
| データ暗号化（primary） | **XChaCha20-Poly1305** | 192bitランダムnonce（誕生日攻撃の余裕が大きい） |
| データ暗号化（互換モード） | AES-256-GCM | 96bitランダムnonce（1Passwordと同一） |
| 乱数 | OS CSPRNG（`getrandom`/`OsRng`） | 自前PRNG禁止 |

Argon2id をprimaryとする理由：1PasswordがPBKDF2に留まるのはWebCrypto制約が
理由であり、本アプリはネイティブのみなので制約がない。メモリハードKDFで
GPUクラック耐性が上がるため設計上「同等以上」。

## 4. 鍵導出：2SKD（Two-Secret Key Derivation）

1Passwordと同一の構成。

- **アカウントパスワード**：ユーザーが記憶する唯一の秘密
- **Secret Key**：128bit（16バイト）。アカウント作成時に端末上で生成。
  ストレージにもVaultファイルにも平文・暗号化済みを問わず含めない。

導出手順：

```
p2s   = random(16)                                # アカウントごとのソルト（保存可）
psalt = HKDF-SHA256(ikm=p2s, salt=user_id, info="2p-salt-v1", len=32)
pkey  = Argon2id(password, psalt, m=64MiB,t=3,p=4, len=32)
skey  = HKDF-SHA256(ikm=secret_key, salt=acct_id, info=sk_version, len=32)
AUK   = pkey XOR skey                             # Account Unlock Key (256bit)
```

- `p2s`, `user_id`, `acct_id`, `sk_version`, KDFパラメータは `profile.json` に平文保存可
- AUKはメモリ上でのみ存在し、ロック時に破棄
- **効果**：Vaultファイルが盗まれてもSecret Keyが不明ならAUKを推測不能
  （パスワード総当たりに加え128bitの秘密が必要）

## 5. 鍵階層

```
アカウントパスワード + Secret Key ──2SKD──> AUK
                                          │
                       暗号化(keyset.json) ▼
                                   Master Keyset
                                   ├─ account signing key（将来の共有用）
                                   └─ vaultごとの鍵をwrap
                                          │
                                          ▼
                                     Vault Key（vaultごと・256bit）
                                          │
                                          ▼
                                   各アイテムの暗号化
```

- 鍵は全てクライアント上で生成
- Vault Key変更＝アイテム再暗号化のみ。Keyset変更はwrapし直し。
  パスワード変更はAUK→Keysetのwrapのみ更新（アイテム無変更で済む設計）

## 6. Vaultファイル形式（v1）

同期競合に強い「アイテム単位ファイル」方式（OPVault式）。

```
vault/
├── profile.json        # version, crypto suite, KDF params, salt, user_id
├── keyset.json         # AUKで暗号化されたmaster keyset
└── items/
    └── <uuid>.json     # アイテムごとに1ファイル
```

`items/<uuid>.json`：

```json
{
  "uuid": "...",
  "v": 1,
  "ct": "base64(nonce || ciphertext)",   // XChaCha20-Poly1305, key=Vault Key
  "aad": "uuid|vault_id|v",              // 改ざん検出用の関連データ
  "tombstone": false,
  "updated_at": "…"                      // 更新時刻は暗号化外（同期用、許容する漏洩）
}
```

- アイテム内容は全て暗号化ペイロード内（タイトル・URL・履歴を含む）
- 平文で漏れるのは uuid・更新時刻・存在の有無のみ
- 削除は tombstone フラグで伝播し、N日後に物理削除

## 7. 同期と端末登録

- サーバー認証を持たないため **SRPは使用しない**（認証相手が存在しない）。
  クラウド盗難への防御は2SKDで担保（§2.2）
- 同期層は「Vaultディレクトリを複製する」抽象インターフェース：
  - macOS/iOS: iCloud Driveコンテナ
  - Windows: iCloud for Windows or OneDrive/Google Driveのフォルダ
  - Android: SAF / Drive API
- 競合解決：アイテム単位の last-write-wins（更新時刻比較）。同一アイテムの
  同時編集は負け側を `.conflict-<ts>` コピーとして保全
- **新規端末の登録**：既存端末からSecret KeyをQRコード表示→新端末でスキャン
  ＋アカウントパスワード入力。紙の「エマージェンシーキット」PDFも生成可

## 8. アプリケーション防御

### 8.1 メモリ

- 秘密は `zeroize` で使用後即消去。コアはRustでメモリ安全性を担保
- mlock/VirtualLockによるスワップ回避はベストエフォート
- 秘密をログ・クラッシュレポート・Analyticsに出さない（クラッシュ収集はopt-in）

### 8.2 ロック

- 自動ロック：アイドルN分・スリープ・画面ロック・手動で即時
- ロック時はAUK・Keyset・平文キャッシュを全てzeroize
- 生体認証アンロック：AUKをハードウェアバインド鍵でwrapして保存
  - iOS/macOS: Keychain `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
    + `biometryCurrentSet`（Secure Enclave）
  - Android: Keystore `setUserAuthenticationRequired(true)` +
    `setInvalidatedByBiometricEnrollment(true)`、StrongBox対応時は使用
  - Windows: Windows Hello (KeyCredential/TPM) or DPAPI `CRYPTPROTECT_LOCAL_MACHINE`

### 8.3 オートフィル・フィッシング耐性

- ドメイン照合はデフォルトで**登録可能ドメイン（eTLD+1）一致**、
  厳密ホスト一致も選択可。Public Suffix List使用
- 不一致サイトへは絶対にフィルしない。IDN/punycode表示の警告
- ブラウザ拡張はnative messaging経由でデスクトップアプリと通信し、
  拡張プロセスに秘密を保持しない（KeePassXC方式。1Passwordより攻撃面が小さい）
- モバイルはOS純正Autofill機構（iOS AutoFill Credential Provider /
  Android Autofill Service）のみ経由

### 8.4 その他

- コピーした秘密はクリップボードを90秒で自動消去（アプリが消去権を持つ場合のみセット）
- Android: `FLAG_SECURE` でスクリーンショット・タスクスイッチャー抑止
- パスワード入力フィールドはスクリーンリーダー等への露出に注意

## 9. 機能別セキュリティ

- **パスワード生成**：CSPRNGのみ。辞書語（EFF large wordlist）パスフレーズと
  文字集合乱数の両対応。生成は全てコアで行いプラットフォーム間で一致させる
- **TOTP**：シードもアイテムとして暗号化。RFC 6238準拠
- **漏洩チェック**：HIBP `/range` APIでSHA-1先頭5文字のみ送信（k-anonymity）。
  パスワード本体・ハッシュ全文は送信しない
- **インポート/エクスポート**：平文エクスポートは明示警告＋一時ファイル即消去。
  CXF（Credential Exchange Format）対応を検討

## 10. 検証・供給連鎖

- 暗号の各ステップ（HKDF、2SKD、AEAD）に**テストベクター**を残し、
  実装変更時の回帰を検出
- KDF・鍵ファイル・パーサに fuzzing（`cargo-fuzz`）
- `cargo audit` / `cargo deny` をCIで常時実行、依存はバージョン固定
- リリースビルドの再現性（可能な範囲で）

## 11. 1Passwordとの既知の差分（正直な記録）

| 項目 | 1Password | 本アプリ |
|---|---|---|
| サーバー認証 | SRP + TLS + transport encryption | なし（サーバーを持たず、ファイル同期） |
| KDF | PBKDF2-650k（WebCrypto制約のため） | Argon2id（上位）、PBKDF2互換モードあり |
| AEAD | AES-256-GCM | XChaCha20-Poly1305（nonce衝突耐性が上） |
| データ復旧 | チームリカバリ（Recovery Group） | なし。エマージェンシーキットのみ |
| 安全な共有 | 公開鍵によるvault共有 | Phase 2以降（個人利用のため優先度低） |
| 保証 | 第三者監査・SOC2・バグバウンティ | 自前テストベクター・fuzzのみ（継続課題） |
| パスキー | プロバイダ対応済み | 保管のみ先行。プロバイダ化は後続 |

## 12. 未決事項

- [x] ~~Secret Key の表示フォーマット~~ → 決定：`P1-` + RFC4648 base32（padなし）
  の26文字を5文字ごとにハイフン区切り（`P1-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-X`）。
  128bitのSecret Keyを忠実にエンコード。実装済み（crypto/secret_key.rs）
- [ ] 同一アイテム同時編集時のマージ戦略（LWW vs フィールド統合）
- [ ] エマージェンシーキットの紙/ファイル形式
- [ ] パスキープロバイダ対応の時期（各OSのAPI権限制約を要調査）
