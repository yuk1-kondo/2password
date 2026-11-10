import SwiftUI

/// Implemented security features, grouped like docs/security-spec.md so the
/// user can verify what actually exists vs marketing claims.
struct SecurityInfoView: View {
    @Environment(\.dismiss) private var dismiss

    private struct Feature: Identifiable {
        var id: String { name }
        let name: String
        let done: Bool
    }

    private let sections: [(String, [Feature])] = [
        ("暗号基盤", [
            .init(name: "2SKD（パスワード + 128bit Secret Key → AUK）", done: true),
            .init(name: "Argon2id KDF（64MiB / t=3 / p=4）", done: true),
            .init(name: "XChaCha20-Poly1305 AEAD", done: true),
            .init(name: "鍵階層 AUK → keyset → vault key", done: true),
            .init(name: "AAD による改ざん検出", done: true),
        ]),
        ("端末保護", [
            .init(name: "Touch ID / Face ID アンロック", done: true),
            .init(name: "Secure Enclave / Keychain 連携", done: true),
            .init(name: "自動ロック（10分・画面ロック・バックグラウンド）", done: true),
            .init(name: "秘密のメモリ即消去（zeroize）", done: true),
            .init(name: "クリップボード90秒自動消去", done: true),
        ]),
        ("同期・データ", [
            .init(name: "iCloud Drive 同期（E2E暗号化ファイル）", done: true),
            .init(name: "競合解決（LWW + tombstone）", done: true),
            .init(name: "アイテム単位ファイル形式", done: true),
        ]),
        ("機能", [
            .init(name: "パスワード / PIN / パスフレーズ生成", done: true),
            .init(name: "TOTP ワンタイムコード", done: true),
            .init(name: "漏洩チェック（HIBP k-anonymity）", done: true),
            .init(name: "お気に入り", done: true),
            .init(name: "検索", done: true),
            .init(name: "CSV インポート / エクスポート", done: true),
            .init(name: "パスワード変更", done: true),
            .init(name: "エマージェンシーキット", done: true),
        ]),
        ("他プラットフォーム", [
            .init(name: "ブラウザ拡張（native messaging基盤）", done: true),
            .init(name: "iOS クライアント", done: true),
            .init(name: "Android クライアント", done: true),
            .init(name: "Windows クライアント（C API準備済み）", done: true),
        ]),
        ("未実装", [
            .init(name: "ブラウザ拡張のリリース版動作", done: false),
            .init(name: "モバイルオートフィル（iOS/Android provider）", done: false),
            .init(name: "パスキー", done: false),
            .init(name: "QR新端末登録（スキャン側）", done: false),
            .init(name: "第三者セキュリティ監査", done: false),
        ]),
    ]

    var body: some View {
        NavigationStack {
            List {
                ForEach(sections, id: \.0) { title, features in
                    Section(title) {
                        ForEach(features) { f in
                            Label {
                                Text(f.name)
                            } icon: {
                                Image(systemName: f.done ? "checkmark.circle.fill" : "circle")
                                    .foregroundStyle(f.done ? .green : .secondary)
                            }
                        }
                    }
                }
            }
            .navigationTitle("実装状況")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("閉じる") { dismiss() }
                }
            }
        }
        .frame(minWidth: 440, minHeight: 520)
    }
}
