import SwiftUI

struct ItemDetailView: View {
    @Environment(VaultStore.self) private var store

    let uuid: String
    @Binding var selection: String?

    @State private var item: ItemDetail?
    @State private var showPassword = false
    @State private var editing = false
    @State private var confirmDelete = false

    var body: some View {
        Group {
            if let item {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        header(item)
                        fields(item)
                    }
                    .padding(24)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .navigationTitle("")
        .toolbar {
            ToolbarItem {
                Button {
                    toggleFavorite()
                } label: {
                    Image(systemName: (item?.favorite ?? false) ? "star.fill" : "star")
                }
            }
            ToolbarItem {
                Button("編集") { editing = true }
            }
            ToolbarItem {
                Button("削除", role: .destructive) { confirmDelete = true }
            }
        }
        .task { await load() }
        .sheet(isPresented: $editing, onDismiss: { Task { await load() } }) {
            ItemEditView(uuid: uuid)
        }
        .alert("このアイテムを削除しますか？", isPresented: $confirmDelete) {
            Button("削除", role: .destructive) {
                try? store.remove(uuid: uuid)
                selection = nil
            }
            Button("キャンセル", role: .cancel) {}
        } message: {
            Text("削除は同期で他の端末にも伝播します。")
        }
    }

    private func header(_ item: ItemDetail) -> some View {
        HStack(spacing: 14) {
            ItemIcon(title: item.title, size: 48)
            VStack(alignment: .leading, spacing: 4) {
                Text(item.title).font(.title2).bold()
                if item.favorite {
                    Label("お気に入り", systemImage: "star.fill")
                        .font(.caption)
                        .foregroundStyle(.yellow)
                }
            }
        }
        .padding(.bottom, 20)
    }

    @ViewBuilder
    private func fields(_ item: ItemDetail) -> some View {
        VStack(spacing: 0) {
            if let username = item.username {
                CopyRow(label: "ユーザー名", value: username)
            }
            if let password = item.password {
                PasswordRow(value: password, revealed: $showPassword)
                BreachRow(password: password)
            }
            if let url = item.url {
                CopyRow(label: "URL", value: url)
            }
            if let totp = item.totpSeed {
                TotpRow(seed: totp)
            }
            if let notes = item.notes, !notes.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("メモ")
                        .font(.caption).foregroundStyle(.secondary)
                    Text(notes)
                        .font(.body)
                        .textSelection(.enabled)
                }
                .padding(.vertical, 12)
            }
        }
    }

    private func toggleFavorite() {
        guard let item else { return }
        let new = NewItem(
            title: item.title,
            username: item.username,
            password: item.password,
            url: item.url,
            notes: item.notes,
            totpSeed: item.totpSeed,
            favorite: !item.favorite
        )
        try? store.update(uuid: uuid, item: new)
        Task { await load() }
    }

    private func load() async {
        item = try? store.detail(uuid: uuid)
    }
}

/// One labelled field with a copy button.
private struct CopyRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 3) {
                Text(label)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(value)
                    .font(.body)
                    .textSelection(.enabled)
            }
            Spacer()
            Button {
                Clipboard.copy(value)
            } label: {
                Image(systemName: "doc.on.doc")
            }
            .buttonStyle(.borderless)
            .help("コピー")
        }
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) {
            Divider()
        }
    }
}

/// HIBP k-anonymity breach check, run in background on appear (spec §9).
private struct BreachRow: View {
    let password: String
    @State private var result: Result<UInt32, Error>?

    var body: some View {
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 3) {
                Text("漏洩チェック")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                switch result {
                case .none:
                    Text("確認中…").font(.caption).foregroundStyle(.secondary)
                case .success(let n) where n == 0:
                    Label("漏洩は見つかりませんでした", systemImage: "checkmark.shield.fill")
                        .font(.callout).foregroundStyle(.green)
                case .success(let n):
                    Label("\(n) 件の漏洩で見つかりました — 変更を推奨", systemImage: "exclamationmark.triangle.fill")
                        .font(.callout).foregroundStyle(.red)
                case .failure:
                    Text("チェックできませんでした（オフライン?）")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
            Spacer()
        }
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) { Divider() }
        .task {
            let pw = password
            result = await Task.detached(priority: .utility) {
                do { return .success(try checkPasswordBreach(password: pw)) }
                catch { return .failure(error) }
            }.value
        }
    }
}

/// Password row with reveal toggle + copy.
private struct PasswordRow: View {
    let value: String
    @Binding var revealed: Bool

    var body: some View {
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 3) {
                Text("パスワード")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(revealed ? value : String(repeating: "•", count: 12))
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
            }
            Spacer()
            Button {
                revealed.toggle()
            } label: {
                Image(systemName: revealed ? "eye.slash" : "eye")
            }
            .buttonStyle(.borderless)
            .help(revealed ? "隠す" : "表示")
            Button {
                Clipboard.copy(value)
            } label: {
                Image(systemName: "doc.on.doc")
            }
            .buttonStyle(.borderless)
            .help("コピー")
        }
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) {
            Divider()
        }
    }
}

/// Live TOTP code with a shrinking countdown ring.
private struct TotpRow: View {
    let seed: String
    @State private var now = Date()

    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    private var remaining: Int {
        30 - Int(now.timeIntervalSince1970) % 30
    }
    private var code: String {
        let ts = UInt64(now.timeIntervalSince1970)
        guard let c = try? totpCode(seed: seed, timestampSecs: ts) else { return "------" }
        return String(c.prefix(3)) + " " + String(c.suffix(3))
    }

    var body: some View {
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 3) {
                Text("ワンタイムパスワード")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(code)
                    .font(.system(.body, design: .monospaced))
            }
            Spacer()
            ZStack {
                Circle()
                    .stroke(Color.secondary.opacity(0.25), lineWidth: 2.5)
                Circle()
                    .trim(from: 0, to: CGFloat(remaining) / 30.0)
                    .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
            .frame(width: 20, height: 20)
            Button {
                Clipboard.copy(code.replacingOccurrences(of: " ", with: ""))
            } label: {
                Image(systemName: "doc.on.doc")
            }
            .buttonStyle(.borderless)
            .help("コピー")
        }
        .padding(.vertical, 10)
        .overlay(alignment: .bottom) {
            Divider()
        }
        .onReceive(timer) { now = $0 }
    }
}
