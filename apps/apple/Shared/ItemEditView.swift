import SwiftUI

struct ItemEditView: View {
    @Environment(VaultStore.self) private var store
    @Environment(\.dismiss) private var dismiss

    /// nil = add mode
    let uuid: String?

    @State private var title = ""
    @State private var username = ""
    @State private var password = ""
    @State private var url = ""
    @State private var notes = ""
    @State private var totpSeed = ""
    @State private var favorite = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("タイトル", text: $title)
                    TextField("ユーザー名", text: $username)
                    LabeledContent("パスワード") {
                        HStack {
                            TextField("パスワード", text: $password)
                                .labelsHidden()
                            Button("生成") {
                                password = (try? generatePassword(
                                    length: 20, digits: true, symbols: true, avoidAmbiguous: false
                                )) ?? password
                            }
                        }
                    }
                    if !password.isEmpty {
                        LabeledContent("強度") {
                            Text(strengthLabel)
                                .font(.callout)
                                .foregroundStyle(strengthColor)
                        }
                    }
                    TextField("URL", text: $url)
                }

                Section("オプション") {
                    TextField("TOTPシード（otpauth:// または base32）", text: $totpSeed)
                    Toggle("お気に入り", isOn: $favorite)
                }

                Section("メモ") {
                    TextField("メモ", text: $notes, axis: .vertical)
                        .labelsHidden()
                        .lineLimit(3...8)
                }

                if let errorMessage {
                    Section {
                        Text(errorMessage).foregroundStyle(.red)
                    }
                }
            }
            .formStyle(.grouped)
            .navigationTitle(uuid == nil ? "アイテム追加" : "アイテム編集")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("キャンセル") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("保存", action: save).disabled(title.isEmpty)
                }
            }
        }
        .frame(minWidth: 520, idealWidth: 560, minHeight: 520, idealHeight: 580)
        .task { await load() }
    }

    /// Rough entropy estimate: pool size ^ length, mapped to a label.
    private var strengthBits: Double {
        var pool = 0
        if password.contains(where: { $0.isLowercase }) { pool += 26 }
        if password.contains(where: { $0.isUppercase }) { pool += 26 }
        if password.contains(where: { $0.isNumber }) { pool += 10 }
        if password.contains(where: { !$0.isLetter && !$0.isNumber }) { pool += 32 }
        guard pool > 0 else { return 0 }
        return Double(password.count) * log2(Double(pool))
    }

    private var strengthLabel: String {
        switch strengthBits {
        case ..<40: return "弱い (\(Int(strengthBits))bit)"
        case ..<70: return "普通 (\(Int(strengthBits))bit)"
        default: return "強い (\(Int(strengthBits))bit)"
        }
    }

    private var strengthColor: Color {
        switch strengthBits {
        case ..<40: return .red
        case ..<70: return .orange
        default: return .green
        }
    }

    private func load() async {
        guard let uuid else { return }
        if let d = try? store.detail(uuid: uuid) {
            title = d.title
            username = d.username ?? ""
            password = d.password ?? ""
            url = d.url ?? ""
            notes = d.notes ?? ""
            totpSeed = d.totpSeed ?? ""
            favorite = d.favorite
        }
    }

    private func save() {
        var normalizedSeed: String? = nil
        if !totpSeed.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            do {
                normalizedSeed = try normalizeTotpSeed(input: totpSeed)
            } catch {
                errorMessage = "TOTPシードの形式が正しくありません（otpauth:// URI または base32）"
                return
            }
        }
        let item = NewItem(
            title: title,
            username: username.isEmpty ? nil : username,
            password: password.isEmpty ? nil : password,
            url: url.isEmpty ? nil : url,
            notes: notes.isEmpty ? nil : notes,
            totpSeed: normalizedSeed,
            favorite: favorite
        )
        do {
            if let uuid {
                try store.update(uuid: uuid, item: item)
            } else {
                try store.add(item)
            }
            dismiss()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
