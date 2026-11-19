import SwiftUI

struct SetupView: View {
    @Environment(VaultStore.self) private var store

    @State private var userId = ""
    @State private var password = ""
    @State private var confirm = ""
    @State private var errorMessage: String?
    @State private var secretKey: String?

    private var formValid: Bool {
        !userId.isEmpty && password.count >= 8 && password == confirm
    }

    var body: some View {
        VaultAccessLayout(
            index: "00 / CREATE",
            title: secretKey == nil ? "自分だけの保管庫。" : "鍵を、安全な場所へ。",
            subtitle: "パスワードも、大切な記録も。\nあなたの手元で守ります。"
        ) {
            if let secretKey {
                secretKeyView(secretKey)
            } else {
                formView
            }
        }
    }

    private var formView: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("アカウントパスワードは復元できません。忘れないものを設定してください。")
                .font(.callout).foregroundStyle(.secondary)
            TextField("アカウントID（メールアドレスなど）", text: $userId)
                .modifier(VaultFieldStyle())
            SecureField("パスワード（8文字以上）", text: $password)
                .modifier(VaultFieldStyle())
            SecureField("パスワードを確認", text: $confirm)
                .modifier(VaultFieldStyle())
            hintText
            if let errorMessage {
                Text(errorMessage).font(.callout).foregroundStyle(.red)
            }
            Button("保管庫を作成 ↗", action: create)
                .buttonStyle(InkButtonStyle())
                .disabled(!formValid)
        }
    }

    @ViewBuilder
    private var hintText: some View {
        if !password.isEmpty && password.count < 8 {
            Text("あと \(8 - password.count) 文字必要です（現在 \(password.count) 文字）")
                .font(.caption)
                .foregroundStyle(.orange)
        } else if !confirm.isEmpty && password != confirm {
            Text("パスワードが一致しません")
                .font(.caption)
                .foregroundStyle(.red)
        } else if password.count >= 8 && confirm.isEmpty {
            Text("確認欄にも同じパスワードを入力してください")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func create() {
        do {
            secretKey = try store.setup(userId: userId, password: password)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func secretKeyView(_ key: String) -> some View {
        VStack(spacing: 16) {
            Image(systemName: "key.fill")
                .font(.system(size: 40))
            Text("Secret Keyを保管してください")
                .font(.title2).bold()
            Text("新しい端末でVaultを開くときに必要です。このキーはこの端末以外には保存されません。紙に写すか、安全な場所に記録してください。")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Text(key)
                .font(.system(.body, design: .monospaced))
                .padding(12)
                .background(.quaternary, in: .rect(cornerRadius: 8))
                .textSelection(.enabled)

            HStack {
                Button("コピー") { Clipboard.copy(key) }
                Button("続ける") { /* state already .unlocked */ }
                    .buttonStyle(InkButtonStyle())
            }
        }
    }
}
