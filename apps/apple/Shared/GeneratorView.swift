import SwiftUI

struct GeneratorView: View {
    @Environment(\.dismiss) private var dismiss

    @State private var length: Double = 20
    @State private var digits = true
    @State private var symbols = true
    @State private var avoidAmbiguous = false
    @State private var usePassphrase = false
    @State private var wordCount: Double = 4
    @State private var output = ""

    var body: some View {
        NavigationStack {
            Form {
                Picker("種類", selection: $usePassphrase) {
                    Text("ランダム").tag(false)
                    Text("パスフレーズ").tag(true)
                }
                .pickerStyle(.segmented)

                if usePassphrase {
                    Stepper("単語数: \(Int(wordCount))", value: $wordCount, in: 3...8)
                } else {
                    Stepper("長さ: \(Int(length))", value: $length, in: 8...64)
                    Toggle("数字", isOn: $digits)
                    Toggle("記号", isOn: $symbols)
                    Toggle("紛らわしい文字を除く (l I 1 O 0)", isOn: $avoidAmbiguous)
                }

                Section("生成結果") {
                    Text(output)
                        .font(.system(.body, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    HStack {
                        Button("再生成", systemImage: "arrow.clockwise", action: regenerate)
                        Button("コピー", systemImage: "doc.on.doc") { Clipboard.copy(output) }
                            .disabled(output.isEmpty)
                    }
                }
            }
            .formStyle(.grouped)
            .navigationTitle("ジェネレータ")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("閉じる") { dismiss() }
                }
            }
        }
        .frame(minWidth: 480, idealWidth: 520, minHeight: 420, idealHeight: 480)
        .task { regenerate() }
        .onChange(of: length) { regenerate() }
        .onChange(of: digits) { regenerate() }
        .onChange(of: symbols) { regenerate() }
        .onChange(of: avoidAmbiguous) { regenerate() }
        .onChange(of: wordCount) { regenerate() }
        .onChange(of: usePassphrase) { regenerate() }
    }

    private func regenerate() {
        if usePassphrase {
            output = (try? generatePassphrase(wordCount: UInt32(wordCount), separator: "-")) ?? ""
        } else {
            output = (try? generatePassword(
                length: UInt32(length),
                digits: digits,
                symbols: symbols,
                avoidAmbiguous: avoidAmbiguous
            )) ?? ""
        }
    }
}
