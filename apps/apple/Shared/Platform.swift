import Foundation

#if os(iOS)
import UIKit
#else
import AppKit
#endif

enum Clipboard {
    /// Seconds a secret may live on the pasteboard (spec §8.4).
    static let clearDelay: Duration = .seconds(90)

    /// Copy a secret, then clear the pasteboard after `clearDelay` — but only
    /// if it still holds our value (another app/user may have replaced it).
    @MainActor
    static func copy(_ string: String) {
#if os(iOS)
        UIPasteboard.general.string = string
        let generation = UIPasteboard.general.changeCount
#else
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(string, forType: .string)
        let generation = NSPasteboard.general.changeCount
#endif
        Task {
            try? await Task.sleep(for: clearDelay)
#if os(iOS)
            let board = UIPasteboard.general
            guard board.changeCount == generation else { return }
            board.string = ""
#else
            let board = NSPasteboard.general
            guard board.changeCount == generation else { return }
            board.clearContents()
#endif
        }
    }
}
