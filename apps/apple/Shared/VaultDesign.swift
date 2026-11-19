import SwiftUI

/// Shared visual language: paper, ink, fine rules and a single muted spectrum.
enum VaultDesign {
    static let ink = Color.primary
    static let muted = Color.secondary
    static let rule = Color.primary.opacity(0.14)
    static let wash = Color.primary.opacity(0.035)
    static let moss = Color(red: 0.37, green: 0.43, blue: 0.27)

    static func caption(_ size: CGFloat = 10) -> Font {
        .system(size: size, weight: .regular, design: .monospaced)
    }
}

struct VaultWordmark: View {
    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 2) {
            Text("2Password").font(.system(size: 23, weight: .semibold)).tracking(-1.2)
            Text("®").font(VaultDesign.caption(11))
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("2Password")
    }
}

struct VaultEyebrow: View {
    let text: String

    var body: some View {
        Text(text)
            .font(VaultDesign.caption())
            .tracking(1.2)
            .foregroundStyle(.secondary)
    }
}

struct InkButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var enabled
    @Environment(\.colorScheme) private var colorScheme

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 14, weight: .medium))
            .frame(maxWidth: .infinity, minHeight: 46)
            .foregroundStyle(colorScheme == .dark ? Color.black : Color.white)
            .background(Color.primary.opacity(enabled ? (configuration.isPressed ? 0.75 : 1) : 0.3))
            .clipShape(.rect(cornerRadius: 2))
            .contentShape(Rectangle())
    }
}

struct VaultFieldStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .textFieldStyle(.plain)
            .padding(14)
            .frame(minHeight: 46)
            .background(VaultDesign.wash)
            .overlay(Rectangle().strokeBorder(VaultDesign.rule, lineWidth: 1))
    }
}

/// A quiet, abstract artwork, drawn locally without network assets or motion.
struct VaultSpectrum: View {
    var compact = false

    var body: some View {
        ZStack {
            LinearGradient(stops: [
                .init(color: Color(red: 0.87, green: 0.87, blue: 0.82), location: 0),
                .init(color: Color(red: 0.79, green: 0.80, blue: 0.66), location: 0.36),
                .init(color: Color(red: 0.88, green: 0.86, blue: 0.48), location: 0.51),
                .init(color: Color(red: 0.40, green: 0.44, blue: 0.48), location: 0.63),
                .init(color: Color(red: 0.24, green: 0.28, blue: 0.36), location: 1)
            ], startPoint: .topLeading, endPoint: .bottomTrailing)
            VStack {
                HStack {
                    Text("PRIVATE BY DESIGN")
                    Spacer()
                    Image(systemName: "arrow.up.right")
                }
                Spacer()
                HStack(alignment: .bottom) {
                    Text("Your digital life.\nQuietly protected.")
                        .font(.system(size: compact ? 23 : 30, weight: .regular)).tracking(-1)
                    Spacer()
                    Text("02 / PW").font(VaultDesign.caption())
                }
            }
            .padding(28)
            .foregroundStyle(Color.white)
            .font(VaultDesign.caption(10))
        }
        .accessibilityHidden(true)
    }
}

struct VaultAccessLayout<Content: View>: View {
    let index: String
    let title: String
    let subtitle: String
    @ViewBuilder let content: Content

    var body: some View {
        GeometryReader { geometry in
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    HStack {
                        VaultWordmark()
                        Spacer()
                        VaultEyebrow(text: "PERSONAL VAULT")
                    }
                    .padding(.bottom, 24)
                    Divider()
                    if geometry.size.width >= 760 {
                        HStack(spacing: 64) {
                            VaultSpectrum().frame(maxWidth: .infinity).frame(height: 440)
                            form.frame(width: 320)
                        }
                        .padding(.vertical, 48)
                    } else {
                        VaultSpectrum(compact: true).frame(height: 180).padding(.top, 24)
                        form.padding(.vertical, 32)
                    }
                    Divider()
                    HStack {
                        Text("あなただけの、大切なもの。")
                        Spacer()
                        Text("2PASSWORD / \(index)")
                    }
                    .font(VaultDesign.caption())
                    .foregroundStyle(.secondary)
                    .padding(.top, 18)
                }
                .padding(geometry.size.width >= 760 ? 40 : 24)
                .frame(maxWidth: 1120)
                .frame(maxWidth: .infinity)
                .frame(minHeight: geometry.size.height, alignment: .center)
            }
        }
    }

    private var form: some View {
        VStack(alignment: .leading, spacing: 24) {
            VaultEyebrow(text: index)
            VStack(alignment: .leading, spacing: 10) {
                Text(title).font(.system(size: 30, weight: .medium)).tracking(-0.8)
                Text(subtitle).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            }
            content
        }
    }
}
