import SwiftUI

/// Monogram icon — deterministic color from the title (djb2 hash), no
/// favicon fetching so site names never leave the device (privacy choice).
struct ItemIcon: View {
    let title: String
    var size: CGFloat = 32

    private var hue: Double {
        var h: UInt64 = 5381
        for c in title.utf8 { h = h &* 33 &+ UInt64(c) }
        return Double(h % 360) / 360.0
    }

    var body: some View {
        Text(title.prefix(1).uppercased())
            .font(.system(size: size * 0.45, weight: .semibold))
            .foregroundStyle(.white)
            .frame(width: size, height: size)
            .background(
                Circle().fill(
                    Color(hue: hue, saturation: 0.55, brightness: 0.8)
                )
            )
    }
}
