import SwiftUI

struct UnlockView: View {
    @Environment(VaultStore.self) private var store

    @State private var password = ""
    @State private var errorMessage: String?
    @State private var busy = false
    @State private var attemptedBiometrics = false

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "lock.fill")
                .font(.system(size: 44))
            Text("2Password")
                .font(.largeTitle).bold()

            SecureField("アカウントパスワード", text: $password)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 320)
                .onSubmit(unlock)

            if let errorMessage {
                Text(errorMessage)
                    .font(.callout)
                    .foregroundStyle(.red)
            }

            Button(action: unlock) {
                if busy { ProgressView() } else { Text("ロック解除") }
            }
            .buttonStyle(.borderedProminent)
            .disabled(password.isEmpty || busy)

            if store.biometricsAvailable {
                Button(action: biometricUnlock) {
                    Label("Touch IDで解除", systemImage: "touchid")
                }
                .disabled(busy)
            }
        }
        .padding(32)
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
