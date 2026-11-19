import SwiftUI

struct UnlockView: View {
    @Environment(VaultStore.self) private var store

    @State private var password = ""
    @State private var errorMessage: String?
    @State private var busy = false
    @State private var attemptedBiometrics = false
    @State private var manualSecretKey = ""

    var body: some View {
        VaultAccessLayout(
            index: "01 / UNLOCK",
            title: "おかえりなさい。",
            subtitle: "大切な情報を、いつもの場所に。\nパスワードで保管庫を開きます。"
        ) {

            SecureField("アカウントパスワード", text: $password)
                .modifier(VaultFieldStyle())
                .onSubmit(unlock)

            if let errorMessage {
                Text(errorMessage)
                    .font(.callout)
                    .foregroundStyle(.red)
            }

            Button(action: unlock) {
                HStack {
                    Text(busy ? "開いています…" : "保管庫を開く")
                    Spacer()
                    Image(systemName: busy ? "hourglass" : "arrow.up.right")
                }
                .padding(.horizontal, 16)
            }
            .buttonStyle(InkButtonStyle())
            .disabled(password.isEmpty || busy)

            if store.biometricsAvailable {
                Button(action: biometricUnlock) {
                    Label("生体認証で開く", systemImage: "touchid")
                }
                .disabled(busy)
            }

            DisclosureGroup("Secret Keyを手入力して復元") {
                VStack(spacing: 8) {
                    TextField("P1-...", text: $manualSecretKey)
                        .modifier(VaultFieldStyle())
                        .font(.system(.caption, design: .monospaced))
                    Button("このSecret Keyで復元", action: unlockWithManualKey)
                        .disabled(manualSecretKey.isEmpty || password.isEmpty || busy)
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .task {
            // Offer biometry once on appearance, like 1Password does.
            if store.biometricsAvailable && !attemptedBiometrics {
                attemptedBiometrics = true
                biometricUnlock()
            }
        }
    }

    private func unlock() {
        busy = true
        errorMessage = nil
        Task {
            do {
                try await store.unlock(password: password)
            } catch is KeychainError {
                errorMessage = "Secret Keyが見つかりません。別端末からの移行時は再登録が必要です"
            } catch {
                errorMessage = "パスワードまたはSecret Keyが違います"
            }
            store.noteUnlockFailure()
            busy = false
        }
    }

    /// Emergency-kit restore: type the Secret Key by hand when the device
    /// lost it (spec §7). Keychain is only written after a successful unlock.
    private func unlockWithManualKey() {
        busy = true
        errorMessage = nil
        Task {
            do {
                try await store.unlockWithSecretKey(
                    password: password, secretKey: manualSecretKey
                )
            } catch {
                errorMessage = "Secret Keyまたはパスワードが違います"
            }
            store.noteUnlockFailure()
            busy = false
        }
    }

    private func biometricUnlock() {
        busy = true
        errorMessage = nil
        Task {
            do {
                try await store.unlockWithBiometrics()
            } catch {
                // Cancelled or unavailable — leave the password form up.
            }
            busy = false
        }
    }
}
