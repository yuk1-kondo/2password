import SwiftUI
import UniformTypeIdentifiers
#if os(macOS)
import AppKit
#endif

struct VaultView: View {
    @Environment(VaultStore.self) private var store

    @State private var category: ItemCategory = .all
    @State private var selection: String?
    @State private var query = ""
    @State private var showingAdd = false
    @State private var showingGenerator = false
    @State private var showingInfo = false
    @State private var showingImporter = false
    @State private var confirmExport = false
    @State private var showingPasswordChange = false
    @State private var showingKit = false
    @State private var ioMessage: String?

    private var filtered: [ItemSummary] {
        store.items
            .filter { category.matches($0) }
            .filter {
                query.isEmpty
                    || $0.title.localizedCaseInsensitiveContains(query)
                    || ($0.username ?? "").localizedCaseInsensitiveContains(query)
                    || ($0.url ?? "").localizedCaseInsensitiveContains(query)
            }
            .sorted { $0.title.localizedCaseInsensitiveCompare($1.title) == .orderedAscending }
    }

    @ToolbarContentBuilder
    private var vaultToolbar: some ToolbarContent {
        ToolbarItem {
            Button("情報", systemImage: "info.circle") { showingInfo = true }
        }
        ToolbarItem {
            Button("ジェネレータ", systemImage: "dice") { showingGenerator = true }
        }
        ToolbarItem {
            Button("再読み込み", systemImage: "arrow.clockwise") { store.reload() }
        }
        ToolbarItem {
            Menu {
                Button("CSVをインポート…") { showingImporter = true }
                Button("CSVをエクスポート…") { confirmExport = true }
                Divider()
                Button("パスワード変更…") { showingPasswordChange = true }
                Button("エマージェンシーキット…") { showingKit = true }
            } label: {
                Image(systemName: "gearshape")
            }
        }
        ToolbarItem {
            Button("追加", systemImage: "plus") { showingAdd = true }
        }
        ToolbarItem {
            Button("ロック", systemImage: "lock") { store.lock() }
        }
    }

    private func categoryRow(_ cat: ItemCategory) -> some View {
        Label { Text(cat.title) } icon: { Image(systemName: cat.icon) }
    }

    private func itemRow(_ item: ItemSummary) -> some View {
        HStack(spacing: 12) {
            ItemIcon(title: item.title)
            VStack(alignment: .leading, spacing: 2) {
                Text(item.title)
                Text(item.username ?? item.url ?? "")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer()
            if item.favorite {
                Image(systemName: "star.fill")
                    .font(.caption)
                    .foregroundStyle(.yellow)
            }
        }
        .padding(.vertical, 4)
    }

    var body: some View {
        NavigationSplitView {
            #if os(macOS)
            List(selection: $category) {
                Section {
                    ForEach(ItemCategory.allCases) { cat in
                        categoryRow(cat).tag(cat)
                    }
                } header: {
                    Text("2Password")
                        .font(.headline)
                }

                Section {
                    Label("\(store.items.count) アイテム", systemImage: "number")
                    Label(store.syncDescription, systemImage: "icloud")
                    Label(
                        BiometricStore.isEnrolled ? "生体認証: 有効" : "生体認証: 未設定",
                        systemImage: "touchid"
                    )
                }
            }
            .listStyle(.sidebar)
            .navigationSplitViewColumnWidth(min: 160, ideal: 180)
            #else
            List {
                Section {
                    ForEach(ItemCategory.allCases) { cat in
                        Button { category = cat } label: {
                            HStack {
                                categoryRow(cat)
                                Spacer()
                                if category == cat { Image(systemName: "checkmark") }
                            }
                        }
                    }
                } header: {
                    Text("2Password")
                        .font(.headline)
                }

                Section {
                    Label("\(store.items.count) アイテム", systemImage: "number")
                    Label(store.syncDescription, systemImage: "icloud")
                    Label(
                        BiometricStore.isEnrolled ? "生体認証: 有効" : "生体認証: 未設定",
                        systemImage: "touchid"
                    )
                }
            }
            .listStyle(.sidebar)
            #endif
        } content: {
            #if os(macOS)
            List(selection: $selection) {
                ForEach(filtered, id: \.uuid) { item in
                    itemRow(item).tag(item.uuid)
                }
            }
            .navigationSplitViewColumnWidth(min: 240, ideal: 280)
            .searchable(text: $query, placement: .sidebar, prompt: "検索")
            .toolbar { vaultToolbar }
            #else
            List {
                ForEach(filtered, id: \.uuid) { item in
                    Button { selection = item.uuid } label: { itemRow(item) }
                }
            }
            .searchable(text: $query, prompt: "検索")
            .toolbar { vaultToolbar }
            #endif
        } detail: {
            if let selection {
                ItemDetailView(uuid: selection, selection: $selection)
            } else {
                ContentUnavailableView("アイテム未選択", systemImage: "lock")
            }
        }
        .sheet(isPresented: $showingAdd) { ItemEditView(uuid: nil) }
        .sheet(isPresented: $showingGenerator) { GeneratorView() }
        .sheet(isPresented: $showingInfo) { SecurityInfoView() }
        .sheet(isPresented: $showingPasswordChange) { PasswordChangeView() }
        .sheet(isPresented: $showingKit) { EmergencyKitView() }
        .fileImporter(isPresented: $showingImporter, allowedContentTypes: [.commaSeparatedText]) { result in
            if case .success(let url) = result {
                do {
                    ioMessage = "\(try store.importCsv(url: url)) 件インポートしました"
                } catch {
                    ioMessage = "インポート失敗: \(error.localizedDescription)"
                }
            }
        }
        .alert("平文でエクスポートしますか？", isPresented: $confirmExport) {
            Button("エクスポート", role: .destructive) { runExport() }
            Button("キャンセル", role: .cancel) {}
        } message: {
            Text("パスワードが誰でも読めるCSVになります。安全な場所に保存し、移行後は削除してください。")
        }
        .alert("インポート/エクスポート", isPresented: .init(
            get: { ioMessage != nil },
            set: { if !$0 { ioMessage = nil } }
        )) {
            Button("OK") { ioMessage = nil }
        } message: {
            Text(ioMessage ?? "")
        }
    }

    private func runExport() {
        #if os(macOS)
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "2password-export.csv"
        panel.allowedContentTypes = [.commaSeparatedText]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            ioMessage = "\(try store.exportCsv(url: url)) 件エクスポートしました"
        } catch {
            ioMessage = "エクスポート失敗: \(error.localizedDescription)"
        }
        #endif
    }
}

enum ItemCategory: Hashable, CaseIterable, Identifiable {
    case all, favorites, logins, notes

    var id: Self { self }

    var title: String {
        switch self {
        case .all: return "すべて"
        case .favorites: return "お気に入り"
        case .logins: return "ログイン"
        case .notes: return "ノート"
        }
    }

    var icon: String {
        switch self {
        case .all: return "tray.full"
        case .favorites: return "star.fill"
        case .logins: return "key.fill"
        case .notes: return "doc.text"
        }
    }

    func matches(_ item: ItemSummary) -> Bool {
        switch self {
        case .all: return true
        case .favorites: return item.favorite
        case .logins: return item.url != nil || item.username != nil
        case .notes: return item.url == nil && item.username == nil
        }
    }
}
