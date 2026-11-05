import SwiftUI

struct PasswordChangeView: View {
    @Environment(VaultStore.self) private var store
    @Environment(\.dismiss) private var dismiss

    @State private var current = ""
    @State private var new = ""
    @State private var confirm = ""
    @State private var errorMessage: String?
    @State private var busy = false

    private var valid: Bool {
        !current.isEmpty && new.count >= 8 && new == confirm && new != current
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("現在のパスワード") {
                    SecureField("現在のパスワード", text: $current)
                }
                Section("新しいパスワード（8文字以上）") {
                    SecureField("新しいパスワード", text: $new)
                    SecureField("新しいパスワード（確認）", text: $confirm)
                }
                if let errorMessage {
                    Text(errorMessage).foregroundStyle(.red)
                }
            }
            .formStyle(.grouped)
            .navigationTitle("パスワード変更")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("キャンセル") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("変更", action: change)
                        .disabled(!valid || busy)
                }
            }
        }
        .frame(minWidth: 440, minHeight: 300)
    }

    private func change() {
        busy = true
        errorMessage = nil
        Task {
            do {
                try await store.changePassword(current: current, new: new)
                dismiss()
            } catch {
                errorMessage = "現在のパスワードが違います"
            }
            busy = false
        }
    }
}
