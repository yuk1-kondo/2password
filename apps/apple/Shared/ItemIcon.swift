import SwiftUI

/// Local typographic monogram; no favicon requests leave the device.
struct ItemIcon: View {
    let title: String
    var size: CGFloat = 32

    var body: some View {
        Text(title.prefix(1).uppercased())
            .font(.system(size: size * 0.45, weight: .semibold))
            .foregroundStyle(.primary)
            .frame(width: size, height: size)
            .background(VaultDesign.wash)
            .overlay(Rectangle().strokeBorder(VaultDesign.rule, lineWidth: 1))
            .accessibilityHidden(true)
    }
}
