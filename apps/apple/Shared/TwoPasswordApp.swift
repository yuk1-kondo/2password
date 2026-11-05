import SwiftUI

@main
struct TwoPasswordApp: App {
    @State private var store = VaultStore()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(store)
#if os(iOS)
                .onChange(of: scenePhase) { _, phase in
                    // Leaving the app locks the vault (spec §8.2).
                    if phase == .background {
                        store.lock()
                    }
                }
#endif
        }
    }
}
