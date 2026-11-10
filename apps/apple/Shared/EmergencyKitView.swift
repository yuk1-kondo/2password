import SwiftUI
#if os(macOS)
import AppKit
#endif
import CoreImage.CIFilterBuiltins

/// Printable emergency kit: account id + Secret Key + recovery instructions
/// (spec §7). The only way back in if all devices are lost.
struct EmergencyKitView: View {
    @Environment(\.dismiss) private var dismiss

    @State private var secretKey: String?
    @State private var userId: String = ""

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 20) {
                Text("エマージェンシーキット")
                    .font(.title2).bold()
                Text("この情報があれば、全ての端末を失ってもVaultを復元できます。印刷して安全な場所に保管してください。")
                    .font(.callout)
                    .foregroundStyle(.secondary)

                GroupBox {
                    VStack(alignment: .leading, spacing: 12) {
                        LabeledContent("アカウントID", value: userId)
                        LabeledContent("Secret Key") {
                            Text(secretKey ?? "…")
                                .font(.system(.body, design: .monospaced))
                                .textSelection(.enabled)
                        }
                        if let secretKey, let qr = qrImage(secretKey) {
                            HStack {
                                Spacer()
                                qr
                                    .resizable()
                                    .interpolation(.none)
                                    .frame(width: 140, height: 140)
                                Spacer()
                            }
                        }
                    }
                    .padding(8)
                }

                HStack {
                    Button("コピー") {
                        Clipboard.copy(
                            "2Password Emergency Kit\nアカウントID: \(userId)\nSecret Key: \(secretKey ?? "")"
                        )
                    }
                    Spacer()
                    Button("閉じる") { dismiss() }
                        .buttonStyle(.borderedProminent)
                }
            }
            .padding(28)
            .frame(width: 480)
            .navigationTitle("エマージェンシーキット")
        }
        .task { await load() }
    }

    /// QR of the Secret Key for new-device enrollment (spec §7 scan side).
    private func qrImage(_ contents: String) -> Image? {
        let context = CIContext()
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(contents.utf8)
        filter.correctionLevel = "M"
        guard let output = filter.outputImage,
              let cg = context.createCGImage(output, from: output.extent)
        else { return nil }
        return Image(decorative: cg, scale: 1)
    }

    private func load() async {
        secretKey = try? KeychainHelper.load()
        // profile.json holds the account id (non-secret)
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        // read via app support dir is unreliable under iCloud location; use the
        // same dir resolution as the store through the FFI-free path.
        var candidates = [
            base.appendingPathComponent("2password/vault/profile.json")
        ]
        #if os(macOS)
        candidates.insert(FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Mobile Documents/com~apple~CloudDocs/2password/vault/profile.json"), at: 0)
        #endif
        for cand in candidates {
            if let data = try? Data(contentsOf: cand),
               let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let uid = obj["user_id"] as? String {
                userId = uid
                break
            }
        }
    }
}
