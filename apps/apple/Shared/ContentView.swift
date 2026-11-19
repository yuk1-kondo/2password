import SwiftUI

struct ContentView: View {
    @Environment(VaultStore.self) private var store

    var body: some View {
        Group {
            switch store.state {
            case .setup:
                SetupView()
            case .locked:
                UnlockView()
            case .unlocked:
                VaultView()
            }
        }
        .tint(VaultDesign.ink)
    }
}
